class InstructorSubmissionsPage {
    constructor() {
        this.courses = [];
        this.assignments = [];
        this.submissions = [];
        this.courseFilter = document.getElementById("submission-course-filter");
        this.assignmentFilter = document.getElementById("submission-assignment-filter");
        this.statusFilter = document.getElementById("submission-status-filter");
        this.studentFilter = document.getElementById("submission-student-filter");
        this.tableBody = document.getElementById("instructor-submissions-body");
        this.reviewModal = bootstrap.Modal.getOrCreateInstance(document.getElementById("submission-review-modal"));
        this.reviewingSubmissionId = null;
    }

    formatDate(value) {
        if (!value) return "-";
        const date = new Date(value);
        return Number.isNaN(date.getTime())
            ? String(value)
            : date.toLocaleString("en-SG", { dateStyle: "medium", timeStyle: "short" });
    }

    populateCourseFilter() {
        this.courseFilter.innerHTML = [
            '<option value="">All courses</option>',
            ...this.courses.map(course =>
                `<option value="${course.course_id}">${HtmlUtils.escape(course.name || "Untitled course")}</option>`
            ),
        ].join("");
    }

    populateAssignmentFilter() {
        const selectedCourseId = Number(this.courseFilter.value);
        const available = selectedCourseId
            ? this.assignments.filter(item => item.courseId === selectedCourseId)
            : this.assignments;
        const previousValue = this.assignmentFilter.value;

        this.assignmentFilter.innerHTML = [
            '<option value="">All assignments</option>',
            ...available.map(item =>
                `<option value="${item.assignment.assignment_id}">${HtmlUtils.escape(item.assignment.title || "Untitled assignment")}</option>`
            ),
        ].join("");

        if ([...this.assignmentFilter.options].some(option => option.value === previousValue)) {
            this.assignmentFilter.value = previousValue;
        }
    }

    filteredSubmissions() {
        const courseId = Number(this.courseFilter.value);
        const assignmentId = Number(this.assignmentFilter.value);
        const status = this.statusFilter.value;
        const query = this.studentFilter.value.trim().toLowerCase();

        return this.submissions.filter(item => {
            const isGraded = item.submission.score !== null && item.submission.score !== undefined;
            const studentText = `${item.submission.student_name} ${item.submission.student_email}`.toLowerCase();

            return (!courseId || item.courseId === courseId)
                && (!assignmentId || item.assignment.assignment_id === assignmentId)
                && (!status || (status === "graded") === isGraded)
                && (!query || studentText.includes(query));
        });
    }

    render() {
        const rows = this.filteredSubmissions();
        const gradedCount = rows.filter(item => item.submission.score !== null && item.submission.score !== undefined).length;

        document.getElementById("submission-total").textContent = rows.length;
        document.getElementById("submission-pending").textContent = rows.length - gradedCount;
        document.getElementById("submission-graded").textContent = gradedCount;
        document.getElementById("submission-result-count").textContent =
            `${rows.length} latest submission${rows.length === 1 ? "" : "s"}`;

        if (!rows.length) {
            this.tableBody.innerHTML = '<tr><td colspan="7" class="text-center text-secondary py-4">No submissions match these filters.</td></tr>';
            return;
        }

        this.tableBody.innerHTML = rows.map(item => {
            const submission = item.submission;
            const isGraded = submission.score !== null && submission.score !== undefined;

            return `
                <tr>
                    <td><strong>${HtmlUtils.escape(submission.student_name)}</strong><br><small class="text-secondary">${HtmlUtils.escape(submission.student_email)}</small></td>
                    <td>${HtmlUtils.escape(item.courseName)}</td>
                    <td>${HtmlUtils.escape(item.assignment.title || "Untitled assignment")}</td>
                    <td>${HtmlUtils.escape(this.formatDate(submission.submitted_at))}</td>
                    <td><span class="badge ${isGraded ? "text-bg-success" : "text-bg-warning"}">${isGraded ? "Graded" : "Awaiting grade"}</span></td>
                    <td>${isGraded ? HtmlUtils.escape(submission.score) : "-"}</td>
                    <td class="text-end"><button class="btn btn-sm btn-outline-dark" type="button" data-review-submission="${submission.submission_id}">Review</button></td>
                </tr>`;
        }).join("");
    }

    getSubmissionItem(submissionId) {
        return this.submissions.find(item => item.submission.submission_id === Number(submissionId));
    }

    safeFileUrl(value) {
        if (!value) return null;

        try {
            const url = new URL(value, window.location.origin);
            return ["http:", "https:"].includes(url.protocol) ? url.href : null;
        } catch (_) {
            return null;
        }
    }

    openReview(submissionId) {
        const item = this.getSubmissionItem(submissionId);
        if (!item) return;

        const { submission, assignment } = item;
        const fileUrl = this.safeFileUrl(submission.file_url);
        const maxScore = assignment.max_score;
        this.reviewingSubmissionId = submission.submission_id;

        document.getElementById("submission-review-context").textContent =
            `${item.courseName} · ${assignment.title || "Untitled assignment"}`;
        document.getElementById("submission-review-student").textContent =
            `${submission.student_name} (${submission.student_email})`;
        document.getElementById("submission-review-date").textContent = this.formatDate(submission.submitted_at);
        document.getElementById("submission-review-text").textContent =
            submission.submission_text || "No submission note provided.";

        const fileLink = document.getElementById("submission-review-file");
        const noFile = document.getElementById("submission-review-no-file");
        fileLink.hidden = !fileUrl;
        noFile.hidden = Boolean(fileUrl);
        fileLink.href = fileUrl || "#";

        const scoreInput = document.getElementById("submission-review-score");
        scoreInput.value = submission.score ?? "";
        scoreInput.max = maxScore ?? "";
        document.getElementById("submission-review-max-score").textContent =
            maxScore !== null && maxScore !== undefined ? `Maximum: ${maxScore}` : "No maximum score set";
        document.getElementById("submission-review-feedback").value = submission.feedback || "";
        document.getElementById("submission-review-status").textContent = "";
        this.reviewModal.show();
    }

    async saveReview() {
        const item = this.getSubmissionItem(this.reviewingSubmissionId);
        if (!item) return;

        const scoreInput = document.getElementById("submission-review-score");
        const feedbackInput = document.getElementById("submission-review-feedback");
        const status = document.getElementById("submission-review-status");
        const saveButton = document.getElementById("submission-review-save");
        const score = Number(scoreInput.value);

        if (scoreInput.value === "" || !Number.isFinite(score) || score < 0) {
            status.className = "small mt-3 mb-0 text-danger";
            status.textContent = "Enter a valid score.";
            return;
        }

        saveButton.disabled = true;
        status.className = "small mt-3 mb-0 text-secondary";
        status.textContent = "Saving grade...";

        try {
            const saved = await LmsApi.put(`/api/submissions/${item.submission.submission_id}/grade`, {
                score,
                feedback: feedbackInput.value.trim() || null,
            });
            item.submission.score = saved.score;
            item.submission.feedback = saved.feedback;
            this.render();
            status.className = "small mt-3 mb-0 text-success";
            status.textContent = "Grade saved successfully.";
        } catch (error) {
            status.className = "small mt-3 mb-0 text-danger";
            status.textContent = error.response?.data || "Unable to save this grade.";
        } finally {
            saveButton.disabled = false;
        }
    }

    bindFilters() {
        this.courseFilter.addEventListener("change", () => {
            this.populateAssignmentFilter();
            this.render();
        });
        this.assignmentFilter.addEventListener("change", () => this.render());
        this.statusFilter.addEventListener("change", () => this.render());
        this.studentFilter.addEventListener("input", () => this.render());
        this.tableBody.addEventListener("click", event => {
            const button = event.target.closest("[data-review-submission]");
            if (button) this.openReview(button.dataset.reviewSubmission);
        });
        document.getElementById("submission-review-save")
            .addEventListener("click", () => this.saveReview());
    }

    async load() {
        try {
            this.courses = await LmsApi.get("/api/courses/organisation");
            this.populateCourseFilter();

            const overviewRows = await Promise.all(this.courses.map(async course => ({
                course,
                overview: await LmsApi.get(`/api/courses/${course.course_id}/overview`),
            })));

            this.assignments = overviewRows.flatMap(({ course, overview }) =>
                (overview.assignments || []).map(assignment => ({
                    courseId: course.course_id,
                    courseName: course.name || "Untitled course",
                    assignment,
                }))
            );
            this.populateAssignmentFilter();

            const submissionGroups = await Promise.all(this.assignments.map(async item => {
                const submissions = await LmsApi.get(`/api/assignments/${item.assignment.assignment_id}/submissions`);
                return submissions
                    .filter(submission => submission.is_latest)
                    .map(submission => ({ ...item, submission }));
            }));

            this.submissions = submissionGroups.flat();
            this.render();
        } catch (error) {
            console.error("Failed to load instructor submissions:", error);
            this.tableBody.innerHTML = '<tr><td colspan="7" class="text-danger py-4">Unable to load submissions right now.</td></tr>';
        }
    }
}

document.addEventListener("DOMContentLoaded", () => {
    const page = new InstructorSubmissionsPage();
    page.bindFilters();
    page.load();
});
