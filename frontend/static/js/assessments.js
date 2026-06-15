// assessments.js — AssessmentsPage class for /assessments.
// Depends on lms-core.js (HtmlUtils, LmsApi, PageState).

class AssessmentsPage {
    constructor() {
        this.state = new PageState("assessments-container");
    }

    // ---------------------------------------------------------------------------
    // Attempt status resolution
    // ---------------------------------------------------------------------------

    resolveStatus(quiz, attemptsForQuiz) {
        const submitted = attemptsForQuiz.filter(a => a.submitted_at !== null);
        const inProgress = attemptsForQuiz.filter(a => a.submitted_at === null);
        const maxAttempts = quiz.max_attempts;

        if (submitted.length > 0 && (maxAttempts === null || submitted.length < maxAttempts)) {
            return { label: "Completed", badgeClass: "bg-success", canAttempt: true };
        }
        if (submitted.length > 0 && maxAttempts !== null && submitted.length >= maxAttempts) {
            return { label: "Limit Reached", badgeClass: "bg-danger", canAttempt: false };
        }
        if (inProgress.length > 0) {
            return { label: "In Progress", badgeClass: "bg-warning text-dark", canAttempt: true };
        }
        return { label: "Not Attempted", badgeClass: "bg-secondary", canAttempt: true };
    }

    // ---------------------------------------------------------------------------
    // Rendering
    // ---------------------------------------------------------------------------

    renderQuizCard(courseId, quiz, allAttempts) {
        const attemptsForQuiz = allAttempts.filter(a => a.quiz_id === quiz.quiz_id);
        const status = this.resolveStatus(quiz, attemptsForQuiz);

        const timeLimitMeta = quiz.time_limit
            ? `<span class="badge bg-light text-dark border me-1">
                   <i class="bi bi-clock me-1"></i>${quiz.time_limit} min
               </span>`
            : "";
        const maxAttemptsMeta = quiz.max_attempts
            ? `<span class="badge bg-light text-dark border me-1">
                   <i class="bi bi-arrow-repeat me-1"></i>${attemptsForQuiz.filter(a => a.submitted_at).length}/${quiz.max_attempts} attempts
               </span>`
            : "";

        let startBtn = "";
        if (status.canAttempt) {
            startBtn = `<a href="/course/${courseId}" class="btn btn-sm btn-dark">
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
                            <span class="badge ${status.badgeClass}">${status.label}</span>
                        </div>
                    </div>
                    <div class="flex-shrink-0">${startBtn}</div>
                </div>
            </div>`;
    }

    renderAccordion(courseQuizGroups, attempts) {
        return courseQuizGroups.map((group, i) => {
            const quizCards = group.quizzes.length
                ? group.quizzes.map(q => this.renderQuizCard(group.courseId, q, attempts)).join("")
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

            const [quizResults, attempts] = await Promise.all([
                Promise.all(
                    courses.map(c =>
                        LmsApi.safeGet(`/api/quiz/${c.course_id}`)
                            .then(data => ({
                                courseId:   c.course_id,
                                courseName: c.name || `Course #${c.course_id}`,
                                quizzes:    data || [],
                            }))
                    )
                ),
                LmsApi.safeGet("/api/quiz-attempts/my"),
            ]);

            const allAttempts = attempts || [];
            const hasQuizzes  = quizResults.some(g => g.quizzes.length > 0);

            if (!hasQuizzes) {
                this.state.empty("No assessments have been posted for your courses yet.", "bi-clipboard");
                return;
            }

            this.state.html(`
                <div class="accordion" id="assessments-accordion">
                    ${this.renderAccordion(quizResults, allAttempts)}
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
