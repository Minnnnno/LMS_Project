use actix_web::HttpResponse;
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct MailRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub is_html: bool,
}

pub fn send_mail_message(mail: MailRequest) -> Result<(), String> {
    let smtp_username =
        std::env::var("SMTP_USERNAME").map_err(|_| "SMTP_USERNAME not found".to_string())?;
    let smtp_password =
        std::env::var("SMTP_PASSWORD").map_err(|_| "SMTP_PASSWORD not found".to_string())?;
    let smtp_host = std::env::var("SMTP_HOST").map_err(|_| "SMTP_HOST not found".to_string())?;

    let mut builder = Message::builder()
        .from(
            smtp_username
                .parse::<Mailbox>()
                .map_err(|err| format!("Invalid SMTP sender email: {}", err))?,
        )
        .to(mail
            .to
            .parse::<Mailbox>()
            .map_err(|err| format!("Invalid recipient email: {}", err))?)
        .subject(&mail.subject);

    if mail.is_html {
        builder = builder.header(ContentType::TEXT_HTML);
    }

    let email = builder
        .body(mail.body)
        .map_err(|err| format!("Failed to build email: {}", err))?;

    let creds = Credentials::new(smtp_username, smtp_password);
    let mailer = SmtpTransport::starttls_relay(&smtp_host)
        .map_err(|err| format!("Failed to connect to SMTP host: {}", err))?
        .credentials(creds)
        .build();

    mailer
        .send(&email)
        .map(|_| ())
        .map_err(|err| format!("Failed to send email: {}", err))
}

pub async fn send_mail(mail: MailRequest) -> HttpResponse {
    match send_mail_message(mail) {
        Ok(_) => HttpResponse::Ok().body("Email sent successfully"),
        Err(err) => HttpResponse::InternalServerError().body(err),
    }
}
