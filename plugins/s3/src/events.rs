use serde::Serialize;

use crate::bindgen;

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
            bindgen::OrthancPluginResourceType_OrthancPluginResourceType_Instance => Self::Instance,
            bindgen::OrthancPluginResourceType_OrthancPluginResourceType_Study => Self::Study,
            bindgen::OrthancPluginResourceType_OrthancPluginResourceType_Series => Self::Series,
            bindgen::OrthancPluginResourceType_OrthancPluginResourceType_Patient => Self::Patient,
            bindgen::OrthancPluginResourceType_OrthancPluginResourceType_None => Self::None,
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
            bindgen::OrthancPluginChangeType_OrthancPluginChangeType_Deleted => Self::Deleted,
            bindgen::OrthancPluginChangeType_OrthancPluginChangeType_NewChildInstance => {
                Self::NewChildInstance
            }
            bindgen::OrthancPluginChangeType_OrthancPluginChangeType_NewInstance => {
                Self::NewInstance
            }
            bindgen::OrthancPluginChangeType_OrthancPluginChangeType_NewSeries => Self::NewSeries,
            bindgen::OrthancPluginChangeType_OrthancPluginChangeType_NewStudy => Self::NewStudy,
            bindgen::OrthancPluginChangeType_OrthancPluginChangeType_NewPatient => Self::NewPatient,
            bindgen::OrthancPluginChangeType_OrthancPluginChangeType_StableStudy => {
                Self::StableStudy
            }
            bindgen::OrthancPluginChangeType_OrthancPluginChangeType_StableSeries => {
                Self::StableSeries
            }
            bindgen::OrthancPluginChangeType_OrthancPluginChangeType_StablePatient => {
                Self::StablePatient
            }
            _ => Self::Other,
        }
    }
}
