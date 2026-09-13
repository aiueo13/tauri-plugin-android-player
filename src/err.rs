#[cfg(not(mobile))]
use std::borrow::Cow;


#[cfg(target_os = "android")] 
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(#[from] tauri::plugin::mobile::PluginInvokeError);

#[cfg(not(target_os = "android"))] 
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(Cow<'static, str>);

impl crate::Error {

    #[cfg(not(target_os = "android"))] 
    pub(crate) const NOT_ANDROID: Self = Self(Cow::Borrowed("not android"));
}

impl serde::Serialize for crate::Error {

    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[cfg(target_os = "android")] {
            serializer.serialize_str(&self.0.to_string())
        }
        #[cfg(not(target_os = "android"))] {
            serializer.serialize_str(&self.0)
        }
    }
}