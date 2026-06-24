use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateOrganisationForm {
    pub org_name: String,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct OrganisationSignupForm {
    #[validate(length(min = 1, message = "Organisation name is required."))]
    pub org_name: String,

    #[validate(length(min = 1, message = "Organisation slug is required."))]
    pub org_slug: String,

    pub org_type: Option<String>,

    pub website_url: Option<String>,

    pub admin_first_name: Option<String>,

    pub admin_last_name: Option<String>,

    #[validate(email(message = "Please enter a valid admin email address."))]
    pub admin_email: Option<String>,

    pub admin_password: Option<String>,

    pub confirm_password: Option<String>,

    #[serde(rename = "g-recaptcha-response")]
    pub recaptcha_response: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MassEnrollForm {
    /// List of user_ids to enroll into the org and assign a role
    #[serde(default)]
    pub user_ids: Vec<i32>,
    /// New users to create, attach to the org, assign a role, and email a temporary password
    #[serde(default)]
    pub new_users: Vec<MassEnrollNewUserForm>,
    /// "Instructor" or "Student"
    pub role: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MassEnrollNewUserForm {
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct InviteInstructorForm {
    #[validate(email(message = "Please enter a valid instructor email address."))]
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AssignCourseInstructorForm {
    pub instructor_id: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateOrgClassForm {
    pub class_name: String,
    #[serde(default)]
    pub course_ids: Vec<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateOrgClassForm {
    pub class_name: Option<String>,
    pub course_ids: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AddClassMembersForm {
    #[serde(default)]
    pub user_ids: Vec<i32>,
    #[serde(default)]
    pub new_users: Vec<MassEnrollNewUserForm>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImportClassMembersForm {
    #[serde(default)]
    pub rows: Vec<ImportClassMemberRow>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ImportClassMemberRow {
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub class_name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CourseInstructorDto {
    pub user_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct CourseInstructorCourseDto {
    pub course_id: i32,
    pub name: String,
    pub instructors: Vec<CourseInstructorDto>,
}

#[derive(Debug, Serialize)]
pub struct CourseInstructorSummaryDto {
    pub courses: Vec<CourseInstructorCourseDto>,
    pub instructors: Vec<CourseInstructorDto>,
}

#[derive(Debug, Serialize)]
pub struct OrgMemberDto {
    pub user_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub roles: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OrgClassCourseDto {
    pub course_id: i32,
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct OrgClassMemberDto {
    pub user_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct OrgClassDto {
    pub class_id: i32,
    pub org_id: i32,
    pub courses: Vec<OrgClassCourseDto>,
    pub class_name: String,
    pub members: Vec<OrgClassMemberDto>,
}

#[derive(Debug, Serialize)]
pub struct OrgClassSummaryDto {
    pub classes: Vec<OrgClassDto>,
    pub courses: Vec<OrgClassCourseDto>,
}
