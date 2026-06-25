// upload.js — ProjectsPage class for /projects.
// Depends on lms-core.js (HtmlUtils, LmsApi, PageState).

// Kept as a standalone helper so other pages can reuse it.
async function uploadFile(file, folder = "lms/uploads") {
    const formData = new FormData();
    formData.append("file", file);
    formData.append("folder", folder);

    const res = await fetch("/api/cloudinary/upload", {
        method: "POST",
        body: formData,
    });

    if (!res.ok) {
        const text = await res.text();
        throw new Error(text || "Upload failed");
    }

    return res.json();
}

// ---------------------------------------------------------------------------
// ProjectsPage
// ---------------------------------------------------------------------------
class ProjectsPage {
    constructor() {
        this.state        = new PageState("projects-container");
        this.activeAssignmentId = null;
        this.activeCourseId     = null;
        this._modal       = null;
    }

    _getModal() {
        if (!this._modal) {
            const el = document.getElementById("project-upload-modal");
            if (el) this._modal = new bootstrap.Modal(el);
        }
        return this._modal;
    }

    // ---------------------------------------------------------------------------
    // Rendering
    // ---------------------------------------------------------------------------

    formatDueDate(dateStr) {
        if (!dateStr) return "";
        const d = new Date(dateStr);
        if (isNaN(d)) return "";
        const past = d < new Date();
        const label = d.toLocaleDateString("en-SG", { day: "numeric", month: "short", year: "numeric" });
        return `<span class="badge ${past ? "bg-danger" : "bg-warning text-dark"}">
            <i class="bi bi-calendar3 me-1"></i>${label}
        </span>`;
    }

    renderProjectItem(assignment, course) {
        const dueLabel = this.formatDueDate(assignment.due_date);

        return `
            <div class="list-group-item list-group-item-action">
                <div class="d-flex align-items-center justify-content-between gap-2 flex-wrap">
                    <div class="min-w-0">
                        <div class="fw-semibold">${HtmlUtils.escape(assignment.title)}</div>
                        <div class="text-muted small mt-1">
                            <i class="bi bi-journal-bookmark me-1"></i>${HtmlUtils.escape(course.name)}
                            ${dueLabel ? `&nbsp;${dueLabel}` : ""}
                        </div>
                        ${assignment.description
                            ? `<div class="text-muted small mt-1">${HtmlUtils.escape(assignment.description)}</div>`
                            : ""}
                    </div>
                    <button class="btn btn-sm btn-dark flex-shrink-0"
                            onclick="window.projectsPage.openDropbox(${assignment.assignment_id}, ${course.id})">
                        <i class="bi bi-upload me-1"></i>Submit
                    </button>
                </div>
            </div>`;
    }

    renderSubmissions(submissions) {
        const el = document.getElementById("project-submission-history");
        if (!el) return;

        if (!submissions.length) {
            el.innerHTML = `<p class="text-muted small mb-0">No previous submissions.</p>`;
            return;
        }

        const rows = submissions.map(s => {
            const date = s.submitted_at
                ? new Date(s.submitted_at).toLocaleString("en-SG", { timeZone: "Asia/Singapore" })
                : "Unknown date";
            const scoreEl = s.score != null
                ? `<span class="badge bg-success ms-2">${s.score}</span>`
                : `<span class="badge bg-secondary ms-2">Ungraded</span>`;
            const fileLink = s.file_url
                ? `<a href="${HtmlUtils.escape(s.file_url)}" target="_blank" class="ms-2 small">
                    <i class="bi bi-file-earmark-arrow-down"></i> File
                   </a>`
                : "";
            return `
                <div class="d-flex align-items-center gap-2 py-1 border-bottom">
                    <i class="bi bi-clock text-muted small"></i>
                    <span class="small text-muted">${date}</span>
                    ${scoreEl}
                    ${fileLink}
                </div>`;
        }).join("");

        el.innerHTML = `
            <p class="fw-semibold small mb-1">Previous Submissions</p>
            ${rows}`;
    }

    setSubmitState(loading, message = "") {
        const btn    = document.getElementById("project-submit-btn");
        const status = document.getElementById("project-upload-status");
        if (btn) {
            btn.disabled    = loading;
            btn.textContent = loading ? "Submitting..." : "Submit";
        }
        if (status) status.textContent = message;
    }

    // ---------------------------------------------------------------------------
    // Modal
    // ---------------------------------------------------------------------------

    async openDropbox(assignmentId, courseId) {
        this.activeAssignmentId = assignmentId;
        this.activeCourseId     = courseId;

        const fileInput = document.getElementById("project-file-input");
        const noteInput = document.getElementById("project-note-input");
        if (fileInput) fileInput.value = "";
        if (noteInput) noteInput.value = "";
        this.setSubmitState(false, "");

        // Load submission history
        const histEl = document.getElementById("project-submission-history");
        if (histEl) histEl.innerHTML = `<p class="text-muted small">Loading history...</p>`;

        this._getModal()?.show();

        const submissions = await LmsApi.safeGet(`/api/assignments/${assignmentId}/submissions/my`);
        this.renderSubmissions(submissions || []);
    }

    async submitProject() {
        const assignmentId = this.activeAssignmentId;
        if (!assignmentId) return;

        const fileInput  = document.getElementById("project-file-input");
        const noteInput  = document.getElementById("project-note-input");
        const file       = fileInput?.files?.[0] || null;
        const noteText   = noteInput?.value?.trim() || null;

        if (!file && !noteText) {
            this.setSubmitState(false, "Please attach a file or add a note before submitting.");
            return;
        }

        this.setSubmitState(true, "Uploading...");

        try {
            let fileUrl     = null;
            let cloudinaryId = null;

            if (file) {
                const uploadResult = await uploadFile(file, "lms/submissions");
                fileUrl      = uploadResult.secure_url;
                cloudinaryId = uploadResult.public_id;
            }

            this.setSubmitState(true, "Saving submission...");

            await LmsApi.post(`/api/assignments/${assignmentId}/submissions`, {
                submission_text:       noteText,
                file_url:              fileUrl,
                cloudinary_public_id:  cloudinaryId,
            });

            this.setSubmitState(false, "Submitted successfully!");

            const submissions = await LmsApi.safeGet(`/api/assignments/${assignmentId}/submissions/my`);
            this.renderSubmissions(submissions || []);
        } catch (error) {
            if (error.response?.status === 401) {
                window.location.href = "/login";
                return;
            }
            this.setSubmitState(false, error.message || "Submission failed. Please try again.");
        }
    }

    // ---------------------------------------------------------------------------
    // Data loading
    // ---------------------------------------------------------------------------

    async load() {
        this.state.loading("Loading your assignments...");

        try {
            const assignmentGroups = await LmsApi.get("/api/my-courses/assignments-overview");

            if (!assignmentGroups.length) {
                this.state.empty("You are not enrolled in any courses yet.", "bi-kanban");
                return;
            }

            const items = assignmentGroups.flatMap(({ course, assignments }) =>
                (assignments || [])
                    .filter(a => a.allow_file_submission)
                    .map(a => this.renderProjectItem(a, new Course(course)))
            );

            if (!items.length) {
                this.state.empty("No file submission assignments have been posted yet.", "bi-kanban");
                return;
            }

            this.state.html(`<div class="list-group list-group-flush">${items.join("")}</div>`);

            // Wire submit button
            document.getElementById("project-submit-btn")
                ?.addEventListener("click", () => this.submitProject());
        } catch (error) {
            LmsApi.handleError(error);
            this.state.error("Unable to load projects. Please try again.");
        }
    }
}

document.addEventListener("DOMContentLoaded", () => {
    if (document.getElementById("projects-container")) {
        window.projectsPage = new ProjectsPage();
        window.projectsPage.load();
    }
});
