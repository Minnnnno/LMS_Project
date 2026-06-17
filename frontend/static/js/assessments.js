// assessments.js — AssessmentsPage class for /assessments.
// Depends on lms-core.js (HtmlUtils, LmsApi, PageState).

class AssessmentsPage {
    constructor() {
        this.state = new PageState("assessments-container");
    }

    // ---------------------------------------------------------------------------
    // Rendering
    // ---------------------------------------------------------------------------

    getStatusBadge(status) {
        if (!status) {
            return { label: "Unavailable", badgeClass: "bg-secondary", canAttempt: false };
        }

        if (!status.can_attempt) {
            return { label: status.message || "Unavailable", badgeClass: "bg-danger", canAttempt: false };
        }

        return { label: status.message || "Available", badgeClass: "bg-success", canAttempt: true };
    }

    renderQuizCard(courseId, quiz, statusesByQuiz) {
        const status = statusesByQuiz[quiz.quiz_id] || null;
        const badge = this.getStatusBadge(status);

        const timeLimitMeta = quiz.time_limit
            ? `<span class="badge bg-light text-dark border me-1">
                   <i class="bi bi-clock me-1"></i>${quiz.time_limit} min
               </span>`
            : "";
        const maxAttemptsMeta = quiz.max_attempts
            ? `<span class="badge bg-light text-dark border me-1">
                   <i class="bi bi-arrow-repeat me-1"></i>${status?.attempts_used ?? 0}/${quiz.max_attempts} attempts
               </span>`
            : "";

        let startBtn = "";
        if (badge.canAttempt) {
            startBtn = `<a href="/course/${courseId}/quiz/${quiz.quiz_id}/attempt" class="btn btn-sm btn-dark">
                <i class="bi bi-play-fill me-1"></i>Start Quiz
            </a>`;
        }

        return `
            <div class="card mb-2 border-0 shadow-sm">
                <div class="card-body d-flex align-items-center gap-3">
                    <div class="flex-shrink-0 text-muted" style="font-size:1.5rem;">
                        <i class="bi bi-clipboard-check"></i>
                    </div>
                    <div class="flex-grow-1 min-w-0">
                        <div class="fw-semibold">${HtmlUtils.escape(quiz.title)}</div>
                        ${quiz.description
                            ? `<div class="text-muted small mb-1">${HtmlUtils.escape(quiz.description)}</div>`
                            : ""}
                        <div class="d-flex flex-wrap gap-1 mt-1">
                            ${timeLimitMeta}
                            ${maxAttemptsMeta}
                            <span class="badge ${badge.badgeClass}">${HtmlUtils.escape(badge.label)}</span>
                        </div>
                    </div>
                    <div class="flex-shrink-0">${startBtn}</div>
                </div>
            </div>`;
    }

    renderAccordion(courseQuizGroups) {
        return courseQuizGroups.map((group, i) => {
            const quizCards = group.quizzes.length
                ? group.quizzes.map(q => this.renderQuizCard(group.courseId, q, group.statusesByQuiz || {})).join("")
                : `<p class="text-muted small py-2 px-1 mb-0">No quizzes in this course yet.</p>`;

            const collapseId = `quiz-accordion-${i}`;
            return `
                <div class="accordion-item border mb-2 rounded-3 overflow-hidden">
                    <h2 class="accordion-header">
                        <button class="accordion-button ${i === 0 ? "" : "collapsed"} fw-semibold"
                                type="button"
                                data-bs-toggle="collapse"
                                data-bs-target="#${collapseId}"
                                aria-expanded="${i === 0 ? "true" : "false"}"
                                aria-controls="${collapseId}">
                            <i class="bi bi-journal-bookmark me-2 text-muted"></i>
                            ${HtmlUtils.escape(group.courseName)}
                            <span class="badge bg-secondary ms-2">${group.quizzes.length}</span>
                        </button>
                    </h2>
                    <div id="${collapseId}" class="accordion-collapse collapse ${i === 0 ? "show" : ""}">
                        <div class="accordion-body pt-2 pb-1">
                            ${quizCards}
                        </div>
                    </div>
                </div>`;
        }).join("");
    }

    // ---------------------------------------------------------------------------
    // Data loading
    // ---------------------------------------------------------------------------

    async load() {
        this.state.loading("Loading your assessments...");

        try {
            const courses = await LmsApi.get("/api/my-courses");

            if (!courses.length) {
                this.state.empty("You are not enrolled in any courses yet.", "bi-clipboard-x");
                return;
            }

            const quizResults = await Promise.all(
                courses.map(async (course) => {
                    const [quizzes, statuses] = await Promise.all([
                        LmsApi.safeGet(`/api/quiz/${course.course_id}`),
                        LmsApi.safeGet(`/api/quiz-attempts/my/course/${course.course_id}/status`),
                    ]);

                    return {
                        courseId: course.course_id,
                        courseName: course.name || `Course #${course.course_id}`,
                        quizzes: quizzes || [],
                        statusesByQuiz: (statuses || []).reduce((map, status) => {
                            map[status.quiz_id] = status;
                            return map;
                        }, {}),
                    };
                })
            );
            const hasQuizzes  = quizResults.some(g => g.quizzes.length > 0);

            if (!hasQuizzes) {
                this.state.empty("No assessments have been posted for your courses yet.", "bi-clipboard");
                return;
            }

            this.state.html(`
                <div class="accordion" id="assessments-accordion">
                    ${this.renderAccordion(quizResults)}
                </div>`);
        } catch (error) {
            LmsApi.handleError(error);
            this.state.error("Unable to load assessments. Please try again.");
        }
    }
}

document.addEventListener("DOMContentLoaded", () => {
    if (document.getElementById("assessments-container")) {
        new AssessmentsPage().load();
    }
});

