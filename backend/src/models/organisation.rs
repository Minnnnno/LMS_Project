use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateOrganisationForm {
    pub org_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MassEnrollForm {
    /// List of user_ids to enroll into the org and assign a role
    pub user_ids: Vec<i32>,
    /// "Instructor" or "Student"
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct OrgMemberDto {
    pub user_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub roles: Vec<String>,
}
