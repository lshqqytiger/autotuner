use crate::parameter::Profile;

pub(crate) trait IntoJson {
    fn into_json(self, profile: &Profile) -> serde_json::Value;
}
