// main.js — Page classes for Dashboard, Certification, and Challenges.
// Depends on lms-core.js (HtmlUtils, Course, LmsApi, PageState).

// ---------------------------------------------------------------------------
// DashboardPage — replaces the inline script in index.html
// ---------------------------------------------------------------------------
class DashboardPage {
    constructor() {
        this.state = new PageState("my-courses-list");
        const el   = document.getElementById("my-courses-list");
        this.mode  = el ? (el.dataset.dashboardMode || "student") : "student";
    }

    renderCourseItem(course) {
        return `
            <a class="my-course-item" href="/course/${course.id}">
                <span class="my-course-thumb"
                      style="background-image: url('${HtmlUtils.escape(course.imageUrl)}')"></span>
                <span class="my-course-body">
                    <span class="my-course-title">${HtmlUtils.escape(course.name)}</span>
                    <span class="my-course-meta">${HtmlUtils.escape(course.formattedPrice)}</span>
                </span>
                <i class="bi bi-chevron-right" aria-hidden="true"></i>
            </a>`;
    }

    async load() {
        this.state.loading("Loading your courses...");

        const endpoint = this.mode === "organisation"
            ? "/api/courses/organisation"
            : "/api/my-courses";

        try {
            const data = await LmsApi.get(endpoint);
            if (!data.length) {
                const msg = this.mode === "organisation"
                    ? "No courses have been created for your organisation yet."
                    : "You are not enrolled in any courses yet.";
                this.state.empty(msg, "bi-journal-bookmark");
                return;
            }
            this.state.html(data.map(d => this.renderCourseItem(new Course(d))).join(""));
        } catch (error) {
            if (error.response?.status === 401) {
                this.state.empty("Sign in to see your courses.", "bi-person-lock");
                return;
            }
            this.state.error("Unable to load your courses right now.");
        }
    }
}

// ---------------------------------------------------------------------------
// CertificationPage — certification.html
// ---------------------------------------------------------------------------
class CertificationPage {
    constructor() {
        this.state = new PageState("certification-container");
    }

    completionSourceLabel(source) {
        if (source === "manual") return "Staff marked";
        if (source === "automatic") return "Automatic";
        return "Completed";
    }

    formatDate(value) {
        if (!value) return "Issue date unavailable";
        const date = new Date(value);
        if (Number.isNaN(date.getTime())) return "Issue date unavailable";
        return date.toLocaleDateString("en-SG", {
            day: "numeric",
            month: "short",
            year: "numeric",
        });
    }

    renderCertItem(row) {
        const course = new Course(row.course || {});
        const certificate = row.certificate || {};
        const source = row.status?.completion_source || certificate.completion_source;
        const url = certificate.verification_url || "#";

        return `
            <div class="certificate-list-item">
                <div class="certificate-thumb"
                     style="background-image:url('${HtmlUtils.escape(course.imageUrl)}')"></div>
                <div class="certificate-body">
                    <div class="certificate-title">${HtmlUtils.escape(course.name)}</div>
                    <div class="certificate-meta">
                        <span><i class="bi bi-patch-check-fill" aria-hidden="true"></i>${HtmlUtils.escape(this.completionSourceLabel(source))}</span>
                        <span><i class="bi bi-calendar3" aria-hidden="true"></i>Issued ${HtmlUtils.escape(this.formatDate(certificate.issued_at))}</span>
                    </div>
                    <div class="certificate-link-text">${HtmlUtils.escape(url)}</div>
                </div>
                <div class="certificate-actions">
                    <a class="btn btn-sm btn-dark" href="${HtmlUtils.escape(url)}" target="_blank" rel="noopener">
                        <i class="bi bi-box-arrow-up-right me-1" aria-hidden="true"></i>View
                    </a>
                    <button class="btn btn-sm btn-outline-dark" type="button" data-copy-certificate="${HtmlUtils.escape(url)}">
                        <i class="bi bi-clipboard me-1" aria-hidden="true"></i>Copy
                    </button>
                </div>
            </div>`;
    }

    async copyCertificateLink(url, button) {
        try {
            await navigator.clipboard.writeText(url);
            const original = button.innerHTML;
            button.innerHTML = `<i class="bi bi-check2 me-1" aria-hidden="true"></i>Copied`;
            window.setTimeout(() => {
                button.innerHTML = original;
            }, 1600);
        } catch (_) {
            window.prompt("Certificate verification link:", url);
        }
    }

    bindCopyActions() {
        if (!this.state.container) return;
        this.state.container.querySelectorAll("[data-copy-certificate]").forEach((button) => {
            button.addEventListener("click", () => {
                this.copyCertificateLink(button.dataset.copyCertificate || "", button);
            });
        });
    }

    async load() {
        this.state.loading("Loading your certifications...");

        try {
            const certificates = await LmsApi.get("/api/certificates/my");

            if (!certificates.length) {
                this.state.empty(
                    "Complete a course to receive a verification link.",
                    "bi-patch-check"
                );
                return;
            }

            const items = certificates
                .map(row => this.renderCertItem(row))
                .join("");

            this.state.html(`<div class="certificate-list">${items}</div>`);
            this.bindCopyActions();
        } catch (error) {
            LmsApi.handleError(error);
            this.state.error("Unable to load certifications. Please try again.");
        }
    }
}

// ---------------------------------------------------------------------------
// CertificateVerificationPage - public verification result
// ---------------------------------------------------------------------------
class CertificateVerificationPage {
    constructor() {
        this.state = new PageState("certificate-verification-container");
    }

    formatDate(value) {
        if (!value) return "Unavailable";
        const date = new Date(value);
        if (Number.isNaN(date.getTime())) return "Unavailable";
        return date.toLocaleDateString("en-SG", {
            day: "numeric",
            month: "short",
            year: "numeric",
        });
    }

    completionSourceLabel(source) {
        if (source === "manual") return "Staff marked";
        if (source === "automatic") return "Automatic";
        return "Completed";
    }

    render(payload) {
        const validClass = payload.valid ? "valid" : "invalid";
        const icon = payload.valid ? "bi-patch-check-fill" : "bi-exclamation-octagon-fill";
        const title = payload.valid ? "Valid certificate" : "Invalid certificate";

        this.state.html(`
            <section class="certificate-verify-card ${validClass}">
                <div class="certificate-verify-status">
                    <i class="bi ${icon}" aria-hidden="true"></i>
                    <span>${title}</span>
                </div>
                <dl class="certificate-verify-grid">
                    <div>
                        <dt>Student</dt>
                        <dd>${HtmlUtils.escape(payload.student_name)}</dd>
                    </div>
                    <div>
                        <dt>Course</dt>
                        <dd>${HtmlUtils.escape(payload.course_name)}</dd>
                    </div>
                    <div>
                        <dt>Issued</dt>
                        <dd>${HtmlUtils.escape(this.formatDate(payload.issued_at))}</dd>
                    </div>
                    <div>
                        <dt>Completion source</dt>
                        <dd>${HtmlUtils.escape(this.completionSourceLabel(payload.completion_source))}</dd>
                    </div>
                </dl>
                ${payload.valid ? "" : `<p class="certificate-invalid-note">This link exists, but the certificate is not currently valid.</p>`}
            </section>
        `);
    }

    async load() {
        const token = window.location.pathname.split("/").filter(Boolean).pop();
        if (!token) {
            this.state.error("Certificate link is missing.");
            return;
        }

        this.state.loading("Checking certificate...");

        try {
            const payload = await LmsApi.get(`/api/certificates/verify/${encodeURIComponent(token)}`);
            this.render(payload);
        } catch (error) {
            if (error.response?.status === 404) {
                this.state.html(`
                    <section class="certificate-verify-card invalid">
                        <div class="certificate-verify-status">
                            <i class="bi bi-exclamation-octagon-fill" aria-hidden="true"></i>
                            <span>Certificate not found</span>
                        </div>
                        <p class="certificate-invalid-note mb-0">This verification link does not match a SkillUp certificate.</p>
                    </section>
                `);
                return;
            }
            this.state.error("Unable to verify this certificate right now.");
        }
    }
}

// ---------------------------------------------------------------------------
// ChallengesPage — challenges.html — shows assignments as practice challenges
// ---------------------------------------------------------------------------
class ChallengesPage {
    constructor() {
        this.state = new PageState("challenges-container");
    }

    formatDueDate(dateStr) {
        if (!dateStr) return null;
        const d = new Date(dateStr);
        if (isNaN(d)) return null;
        const now = new Date();
        const past = d < now;
        const label = d.toLocaleDateString("en-SG", { day: "numeric", month: "short", year: "numeric" });
        return `<span class="badge ${past ? "bg-danger" : "bg-warning text-dark"} ms-1">
            <i class="bi bi-calendar3 me-1"></i>${label}
        </span>`;
    }

    renderChallengeCard(assignment, course) {
        const dueLabel = this.formatDueDate(assignment.due_date);
        const maxScore = assignment.max_score ? `<small class="text-muted">Max score: ${HtmlUtils.escape(String(assignment.max_score))}</small>` : "";

        return `
            <div class="col">
                <div class="card h-100 border shadow-sm">
                    <div class="card-body d-flex flex-column gap-2">
                        <div class="d-flex align-items-start justify-content-between gap-2">
                            <h6 class="card-title mb-0 fw-bold">${HtmlUtils.escape(assignment.title)}</h6>
                            ${dueLabel || ""}
                        </div>
                        ${assignment.description
                            ? `<p class="card-text text-muted small mb-0">${HtmlUtils.escape(assignment.description)}</p>`
                            : ""}
                        <div class="mt-auto d-flex align-items-center justify-content-between pt-2">
                            <small class="text-muted">
                                <i class="bi bi-journal-bookmark me-1"></i>
                                ${HtmlUtils.escape(course.name)}
                            </small>
                            ${maxScore}
                        </div>
                    </div>
                    <div class="card-footer bg-transparent border-top-0 pt-0 pb-3 px-3">
                        <a href="/course/${course.id}" class="btn btn-sm btn-dark w-100">
                            <i class="bi bi-arrow-right me-1"></i>View Challenge
                        </a>
                    </div>
                </div>
            </div>`;
    }

    async load() {
        this.state.loading("Loading challenges...");

        try {
            const assignmentGroups = await LmsApi.get("/api/my-courses/assignments-overview");

            if (!assignmentGroups.length) {
                this.state.empty("Enroll in a course to see challenges.", "bi-trophy");
                return;
            }

            const cards = assignmentGroups
                .flatMap(({ course, assignments }) =>
                    (assignments || []).map(a => this.renderChallengeCard(a, new Course(course)))
                );

            if (!cards.length) {
                this.state.empty("No challenges have been posted yet.", "bi-trophy");
                return;
            }

            this.state.html(`
                <div class="row row-cols-1 row-cols-md-2 g-4">
                    ${cards.join("")}
                </div>`);
        } catch (error) {
            LmsApi.handleError(error);
            this.state.error("Unable to load challenges. Please try again.");
        }
    }
}

// ---------------------------------------------------------------------------
// Boot — each class guards itself to the correct page anchor
// ---------------------------------------------------------------------------
document.addEventListener("DOMContentLoaded", () => {
    if (document.getElementById("my-courses-list")) {
        new DashboardPage().load();
    }
    if (document.getElementById("certification-container")) {
        new CertificationPage().load();
    }
    if (document.getElementById("certificate-verification-container")) {
        new CertificateVerificationPage().load();
    }
    if (document.getElementById("challenges-container")) {
        new ChallengesPage().load();
    }
});
