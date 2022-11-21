use std::{sync::RwLock, time::Duration};

use futures::TryStreamExt;
use rusoto_core::Region;
use rusoto_credential::StaticProvider;
use rusoto_s3::{DeleteObjectRequest, GetObjectRequest, PutObjectRequest, S3Client, S3};
use tracing::{debug, info, warn};

use crate::{bindgen, config::Config};

lazy_static! {
    static ref GLOBAL_STATE: RwLock<AppState> = {
        dotenv::dotenv().ok();
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "s3=debug")
        }
        tracing_subscriber::fmt::init();

        let config: Config = envy::from_env().expect("unable to load s3 configuration");
        info!("config - {:#?}", &config);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("unable to start tokio runtime");

        let app_state = AppState {
            runtime: Some(runtime),
            context: None,
            config,
        };

        RwLock::new(app_state)
    };
}

pub struct AppState {
    runtime: Option<tokio::runtime::Runtime>,
    config: Config,
    context: Option<OrthancContext>,
}

/// Wrapper struct for a callback function whose FFI will be generated automatically by `bindgen`.
#[repr(C)]
struct OnChangeParams {
    callback: bindgen::OrthancPluginOnChangeCallback,
}

#[repr(C)]
struct OrthancPluginStorageArea2Params {
    create: bindgen::OrthancPluginStorageCreate,
    whole: bindgen::OrthancPluginStorageReadWhole,
    range: bindgen::OrthancPluginStorageReadRange,
    remove: bindgen::OrthancPluginStorageRemove,
}

struct OrthancContext(*mut bindgen::OrthancPluginContext);
unsafe impl Send for OrthancContext {}
unsafe impl Sync for OrthancContext {}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn OrthancPluginInitialize(context: *mut bindgen::OrthancPluginContext) -> i32 {
    info!("initializing");

    let mut app_state = GLOBAL_STATE.try_write().expect("unable to obtain lock");
    app_state.context = Some(OrthancContext(context));

    let s3 = S3Client::try_from(&app_state.config).expect("failed to create s3 client");
    let buckets: Vec<_> = app_state
        .runtime
        .as_ref()
        .unwrap()
        .block_on(async move {
            match s3.list_buckets().await {
                Ok(resp) => match resp.buckets {
                    Some(buckets) => Ok(buckets),
                    None => Err("empty body".to_string()),
                },
                Err(e) => Err(format!("{}", e)),
            }
        })
        .expect("unable to discover storage buckets")
        .into_iter()
        .map(|b| b.name.unwrap())
        .collect();

    info!("discovered buckets - {buckets:#?}");

    let params = Box::new(OnChangeParams {
        callback: Some(on_change),
    });

    let params: *const std::ffi::c_void = Box::into_raw(params) as *mut std::ffi::c_void;
    unsafe {
        let invoker = (*context).InvokeService;
        invoker.unwrap()(
            context,
            bindgen::_OrthancPluginService__OrthancPluginService_RegisterOnChangeCallback,
            params,
        );
    }

    info!("successfully registered 'onchange' callbacks");

    let params = Box::new(OrthancPluginStorageArea2Params {
        create: Some(storage_create),
        whole: Some(storage_read_whole),
        range: Some(storage_read_range),
        remove: Some(storage_remove),
    });
    let params: *const std::ffi::c_void = Box::into_raw(params) as *mut std::ffi::c_void;
    unsafe {
        let invoker = (*context).InvokeService;
        invoker.unwrap()(
            context,
            bindgen::_OrthancPluginService__OrthancPluginService_RegisterStorageArea2,
            params,
        );
    }

    info!("successfully registered 'storage' callbacks");

    info!("initialization complete");
    0
}

#[no_mangle]
pub extern "C" fn OrthancPluginFinalize() {
    let mut app_state = GLOBAL_STATE.try_write().expect("unable to obtain lock");
    if let Some(runtime) = app_state.runtime.take() {
        runtime.shutdown_timeout(Duration::from_secs(5));
    }

    //
    // Give background runtime time to clean up
    //
    std::thread::sleep(Duration::from_secs(5));

    info!("finalized");
}

#[no_mangle]
pub extern "C" fn OrthancPluginGetName() -> *const u8 {
    info!("OrthancPluginGetName");
    "s3\0".as_ptr()
}

#[no_mangle]
pub extern "C" fn OrthancPluginGetVersion() -> *const u8 {
    info!("OrthancPluginGetVersion");
    "1.0.0\0".as_ptr()
}

#[repr(C)]
struct CreateBufferParams {
    target: *mut bindgen::OrthancPluginMemoryBuffer64,
    size: usize,
}

extern "C" fn storage_read_range(
    target: *mut bindgen::OrthancPluginMemoryBuffer64,
    uuid: *const ::std::os::raw::c_char,
    plugin_type: bindgen::OrthancPluginContentType,
    range_start: u64,
) -> bindgen::OrthancPluginErrorCode {
    info!("storage_read_whole called {}", plugin_type);
    match GLOBAL_STATE.try_read() {
        Ok(app_state) => {
            info!("aquired lock for storage read range");

            let config = &app_state.config;
            let s3 = S3Client::try_from(config).expect("failed to create s3 client");

            let range_size = unsafe { (*target).size };

            if range_size == 0 {
                info!("empty range size");
                return 0;
            }

            match unsafe { std::ffi::CStr::from_ptr(uuid) }.to_str() {
                Ok(cstr) => {
                    let uuid = cstr.to_string();

                    let get_req = GetObjectRequest {
                        bucket: config.s3_bucket.to_owned(),
                        key: uuid.to_owned(),
                        ..Default::default()
                    };

                    info!("performing get object");
                    let content = app_state.runtime.as_ref().unwrap().block_on(async move {
                        match s3.get_object(get_req).await {
                            Ok(mut resp) => match resp.body.take() {
                                Some(body) => {
                                    Ok(body.map_ok(|b| b.to_vec()).try_concat().await.unwrap())
                                }
                                None => Err("empty body".to_string()),
                            },
                            Err(e) => Err(format!("{}", e)),
                        }
                    });

                    if let Err(e) = content {
                        warn!("{}", e);
                        return bindgen::OrthancPluginErrorCode_OrthancPluginErrorCode_StorageAreaPlugin;
                    }

                    let content = content.unwrap();

                    unsafe {
                        let data = (*target).data as *mut u8;

                        let content = content.as_ptr().add(range_start as usize);
                        std::ptr::copy_nonoverlapping(content, data, range_size as usize);
                    }

                    info!("read ranged object {}", &uuid);

                    0
                }
                Err(e) => {
                    warn!("unable to parse resource_id to Utf8-String - {}", e);
                    bindgen::OrthancPluginErrorCode_OrthancPluginErrorCode_StorageAreaPlugin
                }
            }
        }
        Err(e) => {
            warn!("unable to get application state - {}", e);
            bindgen::OrthancPluginErrorCode_OrthancPluginErrorCode_StorageAreaPlugin
        }
    }
}

extern "C" fn storage_read_whole(
    target: *mut bindgen::OrthancPluginMemoryBuffer64,
    uuid: *const ::std::os::raw::c_char,
    plugin_type: bindgen::OrthancPluginContentType,
) -> bindgen::OrthancPluginErrorCode {
    info!("storage_read_whole called {}", plugin_type);
    match GLOBAL_STATE.try_read() {
        Ok(app_state) => {
            info!("aquired lock for storage read whole");

            let context = app_state.context.as_ref().unwrap().0;
            let config = &app_state.config;
            let s3 = S3Client::try_from(config).expect("failed to create s3 client");

            match unsafe { std::ffi::CStr::from_ptr(uuid) }.to_str() {
                Ok(cstr) => {
                    let uuid = cstr.to_string();
                    let get_req = GetObjectRequest {
                        bucket: config.s3_bucket.to_owned(),
                        key: uuid.to_owned(),
                        ..Default::default()
                    };

                    info!("performing get_object");
                    let content = app_state.runtime.as_ref().unwrap().block_on(async move {
                        match s3.get_object(get_req).await {
                            Ok(mut resp) => match resp.body.take() {
                                Some(body) => {
                                    Ok(body.map_ok(|b| b.to_vec()).try_concat().await.unwrap())
                                }
                                None => Err("empty body".to_string()),
                            },
                            Err(e) => Err(format!("{}", e)),
                        }
                    });

                    if let Err(e) = content {
                        warn!("{}", e);
                        return bindgen::OrthancPluginErrorCode_OrthancPluginErrorCode_StorageAreaPlugin;
                    }

                    let content = content.unwrap();

                    let params = Box::new(CreateBufferParams {
                        target,
                        size: content.len(),
                    });

                    let params: *const std::ffi::c_void =
                        Box::into_raw(params) as *mut std::ffi::c_void;
                    unsafe {
                        let invoker = (*context).InvokeService;
                        invoker.unwrap()(
                            context,
                            bindgen::_OrthancPluginService__OrthancPluginService_CreateMemoryBuffer64,
                            params,
                        );
                        let _ = Box::from_raw(params as *mut std::ffi::c_void);
                    }

                    unsafe {
                        let data = (*target).data as *mut u8;

                        let content_ptr = content.as_ptr();
                        std::ptr::copy_nonoverlapping(content_ptr, data, content.len());
                    }

                    info!("read object {}", &uuid);

                    0
                }
                Err(e) => {
                    warn!("unable to parse resource_id to Utf8-String - {}", e);
                    bindgen::OrthancPluginErrorCode_OrthancPluginErrorCode_StorageAreaPlugin
                }
            }
        }
        Err(e) => {
            warn!("unable to get application state - {}", e);
            bindgen::OrthancPluginErrorCode_OrthancPluginErrorCode_StorageAreaPlugin
        }
    }
}

extern "C" fn storage_remove(
    uuid: *const ::std::os::raw::c_char,
    plugin_type: bindgen::OrthancPluginContentType,
) -> bindgen::OrthancPluginErrorCode {
    info!("storage_remove called {}", plugin_type);

    match GLOBAL_STATE.try_read() {
        Ok(app_state) => {
            info!("aquired lock for storage remove");

            let config = &app_state.config;
            let s3 = S3Client::try_from(config).expect("failed to create s3 client");

            match unsafe { std::ffi::CStr::from_ptr(uuid) }.to_str() {
                Ok(cstr) => {
                    let uuid = cstr.to_string();
                    let put_req = DeleteObjectRequest {
                        bucket: config.s3_bucket.to_owned(),
                        key: uuid.to_owned(),
                        ..Default::default()
                    };

                    let uuid_async = uuid.clone();

                    info!("deleting object");
                    app_state.runtime.as_ref().unwrap().block_on(async move {
                        if let Err(e) = s3.delete_object(put_req).await {
                            warn!(
                                "could not delete instance '{}' from storage - {}",
                                uuid_async, e
                            );
                        }
                    });

                    info!("removed DICOM {}", &uuid);
                    0
                }
                Err(e) => {
                    warn!("unable to parse resource_id to Utf8-String - {}", e);
                    bindgen::OrthancPluginErrorCode_OrthancPluginErrorCode_StorageAreaPlugin
                }
            }
        }
        Err(e) => {
            warn!("unable to get application state - {}", e);
            bindgen::OrthancPluginErrorCode_OrthancPluginErrorCode_StorageAreaPlugin
        }
    }
}

extern "C" fn storage_create(
    uuid: *const ::std::os::raw::c_char,
    content: *const ::std::os::raw::c_void,
    size: i64,
    plugin_type: bindgen::OrthancPluginContentType,
) -> bindgen::OrthancPluginErrorCode {
    info!("storage_create called {}", plugin_type);
    use std::slice::*;

    match GLOBAL_STATE.try_read() {
        Ok(app_state) => {
            info!("aquired lock for storage create");
            let config = &app_state.config;
            let s3 = S3Client::try_from(config).expect("failed to create s3 client");

            match unsafe { std::ffi::CStr::from_ptr(uuid) }.to_str() {
                Ok(cstr) => {
                    let uuid = cstr.to_string();

                    let content = content as *const u8;
                    let safe_content = unsafe { from_raw_parts(content, size as usize) };

                    let put_req = PutObjectRequest {
                        bucket: config.s3_bucket.to_owned(),
                        key: uuid.to_owned(),
                        body: Some(safe_content.to_vec().into()),
                        ..Default::default()
                    };

                    let uuid_async = uuid.clone();
                    info!("uploading object");
                    app_state.runtime.as_ref().unwrap().block_on(async move {
                        if let Err(e) = s3.put_object(put_req).await {
                            warn!(
                                "could not upload instance to storage '{}' from storage - {}",
                                uuid_async, e
                            );
                        }
                    });

                    info!("created DICOM {}", &uuid);
                    0
                }
                Err(e) => {
                    warn!("unable to parse resource_id to Utf8-String - {}", e);
                    bindgen::OrthancPluginErrorCode_OrthancPluginErrorCode_StorageAreaPlugin
                }
            }
        }
        Err(e) => {
            warn!("unable to get application state - {}", e);
            bindgen::OrthancPluginErrorCode_OrthancPluginErrorCode_StorageAreaPlugin
        }
    }
}

extern "C" fn on_change(
    change_type: bindgen::OrthancPluginChangeType,
    resource_type: bindgen::OrthancPluginResourceType,
    resource_id: *const ::std::os::raw::c_char,
) -> bindgen::OrthancPluginErrorCode {
    let resource_id = if resource_id.is_null() {
        None
    } else {
        match unsafe { std::ffi::CStr::from_ptr(resource_id) }.to_str() {
            Ok(cstr) => Some(cstr.to_string()),
            Err(e) => {
                warn!("unable to parse resource_id to Utf8-String - {}", e);
                None
            }
        }
    };

    debug!(
        "received on_change - type {}, resource {}, id {:?}",
        change_type, resource_type, resource_id
    );

    bindgen::OrthancPluginErrorCode_OrthancPluginErrorCode_Success
}

impl TryFrom<&Config> for rusoto_s3::S3Client {
    type Error = Box<dyn std::error::Error>;

    fn try_from(config: &Config) -> Result<Self, Self::Error> {
        Ok(Self::new_with(
            rusoto_core::request::HttpClient::new()?,
            StaticProvider::new(
                config.s3_access_key.to_owned(),
                config.s3_secret_key.to_owned(),
                None,
                None,
            ),
            Region::Custom {
                name: config.s3_region.to_owned(),
                endpoint: config.s3_endpoint.to_owned(),
            },
        ))
    }
}
