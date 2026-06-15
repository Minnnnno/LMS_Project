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

    renderCertItem(course, progress) {
        const pct     = Math.round((progress?.progress_percentage ?? 0));
        const isComplete = pct >= 100;
        const progressBar = isComplete
            ? `<span class="badge bg-success"><i class="bi bi-patch-check-fill me-1"></i>Complete</span>`
            : `
                <div class="progress mt-1" style="height: 6px;" role="progressbar"
                     aria-valuenow="${pct}" aria-valuemin="0" aria-valuemax="100">
                    <div class="progress-bar bg-dark" style="width: ${pct}%"></div>
                </div>
                <small class="text-muted">${pct}% complete</small>`;

        return `
            <div class="list-group-item list-group-item-action d-flex align-items-center gap-3 py-3">
                <div class="cert-thumb flex-shrink-0 rounded-3 bg-light"
                     style="width:3.5rem;height:3.5rem;background-image:url('${HtmlUtils.escape(course.imageUrl)}');background-size:cover;background-position:center;"></div>
                <div class="flex-grow-1 min-w-0">
                    <div class="fw-semibold text-truncate">${HtmlUtils.escape(course.name)}</div>
                    ${progressBar}
                </div>
                ${isComplete ? `<button class="btn btn-sm btn-outline-dark flex-shrink-0"
                    onclick="window.print()">
                    <i class="bi bi-printer me-1"></i>Print
                </button>` : ""}
            </div>`;
    }

    async load() {
        this.state.loading("Loading your certifications...");

        try {
            const courses = await LmsApi.get("/api/my-courses");

            if (!courses.length) {
                this.state.empty(
                    "You are not enrolled in any courses yet.",
                    "bi-patch-check"
                );
                return;
            }

            const progresses = await Promise.all(
                courses.map(c =>
                    LmsApi.safeGet(`/api/courses/${c.course_id}/progress`)
                        .then(p => ({ courseId: c.course_id, data: p }))
                )
            );
            const progressMap = Object.fromEntries(
                progresses.map(p => [p.courseId, p.data])
            );

            const items = courses
                .map(c => this.renderCertItem(new Course(c), progressMap[c.course_id]))
                .join("");

            this.state.html(`<div class="list-group list-group-flush">${items}</div>`);
        } catch (error) {
            LmsApi.handleError(error);
            this.state.error("Unable to load certifications. Please try again.");
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
            const courses = await LmsApi.get("/api/my-courses");

            if (!courses.length) {
                this.state.empty("Enroll in a course to see challenges.", "bi-trophy");
                return;
            }

            const assignmentGroups = await Promise.all(
                courses.map(c =>
                    LmsApi.safeGet(`/api/assignment/${c.course_id}`)
                        .then(assignments => ({ course: new Course(c), assignments: assignments || [] }))
                )
            );

            const cards = assignmentGroups
                .flatMap(({ course, assignments }) =>
                    assignments.map(a => this.renderChallengeCard(a, course))
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
    if (document.getElementById("challenges-container")) {
        new ChallengesPage().load();
    }
});
