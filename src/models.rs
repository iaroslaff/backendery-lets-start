use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub struct LetsStartForm {
    #[validate(email(message = "Incorrect @main"))]
    pub email: String,

    #[validate(range(
        min = 1_000,
        exclusive_max = 50_000,
        message = "The value of the field goes out of range"
    ))]
    pub min_budget: u16,

    #[validate(range(
        min = 1_000,
        max = 50_000,
        message = "The value of the field goes out of range"
    ))]
    pub max_budget: u16,

    #[validate(length(
        min = 2,
        max = 32,
        message = "The length of the field goes out of bounds"
    ))]
    pub name: String,

    #[validate(length(
        min = 64,
        max = 512,
        message = "The length of the field goes out of bounds"
    ))]
    pub project_description: String,
}
