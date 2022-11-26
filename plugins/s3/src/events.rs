use serde::Serialize;

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    None,
    Instance,
    Series,
    Study,
    Patient,
    Other,
}

/// Map `OrthancPluginResourceType` in Orthanc Plugin SDK into Rust world.
impl From<std::os::raw::c_uint> for ResourceType {
    fn from(code: std::os::raw::c_uint) -> Self {
        match code {
            orthanc_plugin_bindings::OrthancPluginResourceType_OrthancPluginResourceType_Instance => Self::Instance,
            orthanc_plugin_bindings::OrthancPluginResourceType_OrthancPluginResourceType_Study => Self::Study,
            orthanc_plugin_bindings::OrthancPluginResourceType_OrthancPluginResourceType_Series => Self::Series,
            orthanc_plugin_bindings::OrthancPluginResourceType_OrthancPluginResourceType_Patient => Self::Patient,
            orthanc_plugin_bindings::OrthancPluginResourceType_OrthancPluginResourceType_None => Self::None,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Deleted,
    NewChildInstance,
    NewInstance,
    NewSeries,
    NewStudy,
    NewPatient,
    StableSeries,
    StableStudy,
    StablePatient,
    Other,
}

impl From<std::os::raw::c_uint> for ChangeType {
    fn from(code: std::os::raw::c_uint) -> Self {
        match code {
            orthanc_plugin_bindings::OrthancPluginChangeType_OrthancPluginChangeType_Deleted => Self::Deleted,
            orthanc_plugin_bindings::OrthancPluginChangeType_OrthancPluginChangeType_NewChildInstance => {
                Self::NewChildInstance
            }
            orthanc_plugin_bindings::OrthancPluginChangeType_OrthancPluginChangeType_NewInstance => {
                Self::NewInstance
            }
            orthanc_plugin_bindings::OrthancPluginChangeType_OrthancPluginChangeType_NewSeries => Self::NewSeries,
            orthanc_plugin_bindings::OrthancPluginChangeType_OrthancPluginChangeType_NewStudy => Self::NewStudy,
            orthanc_plugin_bindings::OrthancPluginChangeType_OrthancPluginChangeType_NewPatient => Self::NewPatient,
            orthanc_plugin_bindings::OrthancPluginChangeType_OrthancPluginChangeType_StableStudy => {
                Self::StableStudy
            }
            orthanc_plugin_bindings::OrthancPluginChangeType_OrthancPluginChangeType_StableSeries => {
                Self::StableSeries
            }
            orthanc_plugin_bindings::OrthancPluginChangeType_OrthancPluginChangeType_StablePatient => {
                Self::StablePatient
            }
            _ => Self::Other,
        }
    }
}
