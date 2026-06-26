// Assignment list, dropbox, and submission grading behavior for course details.
async function loadAssignments() {
    try {
        const response = await axios.get("/api/assignment/" + courseId);
        renderAssignments(response.data);
    } catch (error) {
        renderAssignmentsError();
        console.error("Failed to load assignments:", error);
    }
}

function renderAssignments(assignments) {
    currentAssignments = Array.isArray(assignments) ? assignments : [];
    const assignmentList = document.getElementById("assignment-list");
    renderDropboxAssignments();
    if (document.querySelector('.course-tab.active')?.dataset.courseTab === "submissions") {
        renderCourseSubmissionsTab();
    }

    if (!assignmentList) {
        return;
    }

    assignmentList.innerHTML = "";

    if (!currentAssignments.length) {
        assignmentList.innerHTML = isInstructor
            ? '<p class="assignment-empty">No assignments yet. Use Add Assignment to create one.</p>'
            : '<p class="assignment-empty">No tasks due.</p>';
        return;
    }

    assignmentList.innerHTML = currentAssignments.map((assignment) => {
        const moduleLabel = getAssignmentModuleTitle(assignment);
        const prerequisite = getFirstIncompleteAssignmentPrerequisite(assignment);
        const lockHint = !isInstructor && prerequisite
            ? `<div class="assignment-lock-hint">Complete ${escapeHtml(prerequisite.title || "the prerequisite module")} first</div>`
            : "";
        const adminButtons = isInstructor
            ? `
                <div class="module-actions">
                    <button class="module-action-btn edit-btn" onclick="editAssignment(event, ${assignment.assignment_id})">Edit</button>
                    <button class="module-action-btn delete-btn" onclick="deleteAssignment(event, ${assignment.assignment_id})">Delete</button>
                </div>
            `
            : "";

        return `
            <div class="assignment-row" onclick="openAssignmentDetails(${assignment.assignment_id})">
                <div>
                    <div class="assignment-title">${escapeHtml(assignment.title || "Untitled assignment")}</div>
                    <div class="assignment-subtitle">${escapeHtml(moduleLabel)} · Due: ${escapeHtml(formatAssignmentDate(assignment.due_date))}</div>
                    ${lockHint}
                </div>
                ${adminButtons}
            </div>
        `;
    }).join("");
}

function renderAssignmentsError() {
    currentAssignments = [];
    renderDropboxAssignments();
    const assignmentList = document.getElementById("assignment-list");

    if (assignmentList) {
        assignmentList.innerHTML = "<p>No tasks due.</p>";
    }
}

function renderDropboxAssignments() {
    const list = document.getElementById("dropbox-assignment-list");

    if (!list) {
        return;
    }

    const assignments = currentAssignments.filter((assignment) => assignment.allow_file_submission ?? true);

    if (!assignments.length) {
        list.innerHTML = '<p class="grades-empty">No assignment dropboxes are available for this course.</p>';
        return;
    }

    list.innerHTML = assignments.map((assignment) => {
        const prerequisite = getFirstIncompleteAssignmentPrerequisite(assignment);
        const status = prerequisite
            ? `Locked: complete ${prerequisite.title || "the prerequisite module"}`
            : `Due: ${formatAssignmentDate(assignment.due_date)}`;

        return `
            <div class="grade-row dropbox-row" onclick="openAssignmentDropbox(${assignment.assignment_id})">
                <div>
                    <div class="grade-title">${escapeHtml(assignment.title || "Untitled assignment")}</div>
                    <div class="grade-meta">${escapeHtml(getAssignmentModuleTitle(assignment))} · ${escapeHtml(status)}</div>
                </div>
                <div class="grade-score">${prerequisite ? "Locked" : "Open"}</div>
            </div>
        `;
    }).join("");
}

async function loadGrades() {
    const gradeList = document.getElementById("grades-list");
    const summary = document.getElementById("grades-summary");

    if (gradeList) {
        gradeList.innerHTML = '<p class="grades-empty">Loading grades...</p>';
    }

    if (summary) {
        summary.textContent = "";
    }

    try {
        const response = await axios.get(`/api/courses/${courseId}/grades`);
        renderGrades(response.data);
        gradesLoaded = true;
    } catch (error) {
        if (error.response?.status === 401) {
            window.location.href = "/login";
            return;
        }

        const message = error.response?.data || "Failed to load grades.";
        if (gradeList) {
            gradeList.innerHTML = `<p class="grades-error">${escapeHtml(message)}</p>`;
        }
    }
}

function setAssignmentModalTab(tabName) {
    document.querySelectorAll(".assignment-modal-tab").forEach((tab) => {
        tab.classList.toggle("active", tab.dataset.assignmentTab === tabName);
    });

    document.getElementById("assignment-details-tab-panel")
        ?.classList.toggle("active", tabName === "details");
    document.getElementById("assignment-dropbox-tab-panel")
        ?.classList.toggle("active", tabName === "dropbox");
    document.getElementById("assignment-submissions-tab-panel")
        ?.classList.toggle("active", tabName === "submissions");

    if (tabName === "dropbox" && currentAssignmentDetailsId) {
        loadAssignmentSubmissions(currentAssignmentDetailsId);
    }

    if (tabName === "submissions" && currentAssignmentDetailsId) {
        loadStaffAssignmentSubmissions(currentAssignmentDetailsId);
    }
}

function renderCourseSubmissionsTab() {
    const list = document.getElementById("course-submissions-list");

    if (!list) {
        return;
    }

    if (!isInstructor) {
        list.innerHTML = '<p class="grades-empty">Student submissions are available to course staff.</p>';
        return;
    }

    if (!currentAssignments.length) {
        list.innerHTML = '<p class="grades-empty">No assignments have been created yet.</p>';
        return;
    }

    list.innerHTML = currentAssignments.map((assignment) => `
        <div class="grade-row submission-shortcut-row" onclick="openAssignmentDetails(${assignment.assignment_id}, 'submissions')">
            <div>
                <div class="grade-title">${escapeHtml(assignment.title || "Untitled assignment")}</div>
                <div class="grade-meta">${escapeHtml(getAssignmentModuleTitle(assignment))} · Due: ${escapeHtml(formatAssignmentDate(assignment.due_date))}</div>
            </div>
            <div class="grade-score">View Submissions</div>
        </div>
    `).join("");
}

function openAssignmentDropbox(assignmentId) {
    openAssignmentDetails(assignmentId, "dropbox");
}

function openAssignmentDetails(assignmentId, initialTab = "details") {
    const assignment = currentAssignments.find((item) => item.assignment_id === assignmentId);

    if (!assignment) {
        return;
    }

    currentAssignmentDetailsId = assignmentId;
    const submissionsTab = document.getElementById("assignment-submissions-tab-btn");
    if (submissionsTab) {
        submissionsTab.style.display = isInstructor ? "inline-flex" : "none";
    }
    document.getElementById("assignment-details-title").textContent = assignment.title || "Assignment Details";
    document.getElementById("assignment-details-description").textContent =
        assignment.description || "No description provided.";
    document.getElementById("assignment-details-module").textContent = getAssignmentModuleTitle(assignment);
    document.getElementById("assignment-details-due").textContent = formatAssignmentDate(assignment.due_date);
    document.getElementById("assignment-details-score").textContent = assignment.max_score ?? "Not set";
    document.getElementById("assignment-details-passing").textContent =
        assignment.passing_mark !== null && assignment.passing_mark !== undefined
            ? `${formatGradeNumber(assignment.passing_mark)}%`
            : "50%";
    document.getElementById("assignment-details-file-type").textContent =
        getFileTypeLabel(assignment.expected_file_type);
    document.getElementById("assignment-details-submission").textContent =
        getAssignmentSubmissionLabel(assignment);
    document.getElementById("assignment-details-prerequisites").textContent =
        getAssignmentPrerequisiteLabel(assignment);

    const briefWrap = document.getElementById("assignment-details-brief-wrap");
    const briefLink = document.getElementById("assignment-details-brief-link");
    if (assignment.assignment_brief_url) {
        briefLink.href = assignment.assignment_brief_url;
        briefWrap.style.display = "block";
    } else {
        briefLink.href = "#";
        briefWrap.style.display = "none";
    }

    const instructionsWrap = document.getElementById("assignment-details-instructions-wrap");
    const instructions = document.getElementById("assignment-details-instructions");
    if (assignment.submission_instructions) {
        instructions.textContent = assignment.submission_instructions;
        instructionsWrap.style.display = "block";
    } else {
        instructions.textContent = "";
        instructionsWrap.style.display = "none";
    }

    resetAssignmentDropbox(assignment);
    setAssignmentModalTab(isInstructor && initialTab === "dropbox" ? "submissions" : initialTab);
    document.getElementById("assignment-details-modal").style.display = "flex";
}

function closeAssignmentDetails() {
    currentAssignmentDetailsId = null;
    document.getElementById("assignment-details-modal").style.display = "none";
}

function resetAssignmentDropbox(assignment) {
    const fileInput = document.getElementById("assignment-dropbox-file-input");
    const noteInput = document.getElementById("assignment-dropbox-note-input");
    const status = document.getElementById("assignment-dropbox-status");
    const submitButton = document.getElementById("submit-assignment-dropbox-btn");
    const existing = document.getElementById("assignment-dropbox-existing");
    const acceptsFiles = assignment.allow_file_submission ?? true;
    const prerequisite = getFirstIncompleteAssignmentPrerequisite(assignment);
    const isLocked = !isInstructor && Boolean(prerequisite);

    if (fileInput) {
        fileInput.value = "";
        fileInput.disabled = !acceptsFiles || isLocked;
        fileInput.accept = getAssignmentFileAccept(assignment.expected_file_type);
    }

    if (noteInput) {
        noteInput.value = "";
        noteInput.disabled = assignment.allow_text_submission === false || isLocked;
    }

    if (status) {
        status.textContent = isLocked
            ? `Complete ${prerequisite.title || "the prerequisite module"} before submitting this assignment.`
            : acceptsFiles
            ? ""
            : "This assignment is not accepting file uploads.";
    }

    if (submitButton) {
        submitButton.disabled = !acceptsFiles || isLocked;
        submitButton.textContent = isLocked ? "Locked" : "Submit Assignment";
    }

    if (existing) {
        existing.innerHTML = '<span>No submissions uploaded yet.</span>';
    }
}

function getAssignmentFileAccept(fileType) {
    const accepts = {
        pdf: ".pdf,application/pdf",
        docx: ".docx,application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        pptx: ".pptx,application/vnd.openxmlformats-officedocument.presentationml.presentation",
        xlsx: ".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        zip: ".zip,application/zip,application/x-zip-compressed",
        image: "image/*",
    };

    return accepts[fileType] || "";
}

function getFileExtension(fileName) {
    const parts = String(fileName || "").toLowerCase().split(".");
    return parts.length > 1 ? parts.pop() : "";
}

function doesFileMatchExpectedType(file, expectedFileType) {
    if (!expectedFileType || !file) {
        return true;
    }

    const extension = getFileExtension(file.name);
    const contentType = String(file.type || "").toLowerCase();

    const extensionMatches = {
        pdf: extension === "pdf",
        docx: extension === "docx",
        pptx: extension === "pptx",
        xlsx: extension === "xlsx",
        zip: extension === "zip",
        image: ["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg"].includes(extension),
    };

    const contentTypeMatches = {
        pdf: contentType === "application/pdf",
        docx: contentType === "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        pptx: contentType === "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        xlsx: contentType === "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        zip: contentType === "application/zip" || contentType === "application/x-zip-compressed",
        image: contentType.startsWith("image/"),
    };

    return Boolean(extensionMatches[expectedFileType]) &&
        (!contentType || Boolean(contentTypeMatches[expectedFileType]));
}

function validateDropboxFile(assignment, file) {
    if (!file) {
        return null;
    }

    if (assignment.expected_file_type && !doesFileMatchExpectedType(file, assignment.expected_file_type)) {
        return `File type must match ${getFileTypeLabel(assignment.expected_file_type)}.`;
    }

    if (assignment.max_file_size_mb && file.size > Number(assignment.max_file_size_mb) * 1024 * 1024) {
        return `File must be ${assignment.max_file_size_mb} MB or smaller.`;
    }

    return null;
}

function renderSubmissionHistory(submissions) {
    if (!submissions.length) {
        return '<span>No submissions uploaded yet.</span>';
    }

    return `
        <strong>Past submissions</strong>
        <div class="dropbox-history-list">
            ${submissions.map((submission, index) => {
                const submittedAt = formatAssignmentDate(submission.submitted_at);
                const fileLink = submission.file_url
                    ? `<a href="${escapeHtml(submission.file_url)}" target="_blank" rel="noopener">Open submitted file</a>`
                    : "<span>No file attached.</span>";
                const note = submission.submission_text
                    ? `<p class="dropbox-history-note">${escapeHtml(submission.submission_text)}</p>`
                    : "";
                const grade = submission.score !== null && submission.score !== undefined
                    ? `<span class="dropbox-history-pill">Score: ${escapeHtml(formatGradeNumber(submission.score))}</span>`
                    : index === 0
                        ? `<span class="dropbox-history-pill pending">Pending grade</span>`
                        : "";
                const feedback = submission.feedback
                    ? `<p class="dropbox-history-note"><strong>Feedback:</strong> ${escapeHtml(submission.feedback)}</p>`
                    : "";

                return `
                    <div class="dropbox-history-item">
                        <div class="dropbox-history-head">
                            <span>Submission ${submissions.length - index}</span>
                            ${grade}
                        </div>
                        <span class="dropbox-history-date">Submitted: ${escapeHtml(submittedAt)}</span>
                        ${fileLink}
                        ${note}
                        ${feedback}
                    </div>
                `;
            }).join("")}
        </div>
    `;
}

async function loadAssignmentSubmissions(assignmentId) {
    const existing = document.getElementById("assignment-dropbox-existing");

    if (!existing) {
        return;
    }

    existing.innerHTML = "<span>Checking past submissions...</span>";

    try {
        const response = await axios.get(`/api/assignments/${assignmentId}/submissions/my`);
        existing.innerHTML = renderSubmissionHistory(response.data || []);
    } catch (error) {
        existing.innerHTML = '<span>Past submissions could not be loaded.</span>';
    }
}

function getSubmissionStudentKey(submission) {
    return String(submission.user_id ?? submission.student_email ?? submission.submission_id);
}

function getSubmissionStudentDomKey(submission) {
    return getSubmissionStudentKey(submission).replace(/[^a-zA-Z0-9_-]/g, "_");
}

function groupSubmissionsByStudent(submissions) {
    const groups = new Map();

    submissions.forEach((submission) => {
        const key = getSubmissionStudentKey(submission);
        const group = groups.get(key) || [];
        group.push(submission);
        groups.set(key, group);
    });

    return [...groups.values()].map((group) => {
        const sorted = group.slice().sort((first, second) => {
            const dateDiff = new Date(second.submitted_at).getTime() - new Date(first.submitted_at).getTime();

            if (dateDiff !== 0) {
                return dateDiff;
            }

            return Number(second.submission_id) - Number(first.submission_id);
        });
        const latest = sorted.find((submission) => submission.is_latest) || sorted[0];

        return {
            latest,
            past: sorted.filter((submission) => submission.submission_id !== latest.submission_id),
        };
    });
}

function renderSubmissionFileLink(submission) {
    return submission.file_url
        ? `<a href="${escapeHtml(submission.file_url)}" target="_blank" rel="noopener">Open submitted file</a>`
        : "<span>No file attached.</span>";
}

function renderStaffSubmissionReadonlyGrade(submission, maxScore) {
    const hasScore = submission.score !== null && submission.score !== undefined;
    const scoreLabel = hasScore
        ? `Score: ${escapeHtml(formatGradeScore(submission.score, maxScore))}`
        : "Not graded";
    const feedback = submission.feedback
        ? `<p class="dropbox-history-note"><strong>Feedback:</strong> ${escapeHtml(submission.feedback)}</p>`
        : "";

    return `
        <div class="staff-grade-readonly">
            <span>${scoreLabel}</span>
            ${feedback}
        </div>
    `;
}

function renderStaffPastSubmissions(pastSubmissions, maxScore, studentKey) {
    if (!pastSubmissions.length) {
        return "";
    }

    return `
        <button type="button" class="staff-past-toggle" onclick="togglePastSubmissions('${escapeHtml(studentKey)}')">
            View past submissions (${pastSubmissions.length})
        </button>
        <div id="staff-past-submissions-${escapeHtml(studentKey)}" class="staff-past-submissions" hidden>
            ${pastSubmissions.map((submission) => {
                const submittedAt = formatAssignmentDate(submission.submitted_at);
                const note = submission.submission_text
                    ? `<p class="dropbox-history-note">${escapeHtml(submission.submission_text)}</p>`
                    : "";

                return `
                    <div class="staff-past-submission">
                        <div class="dropbox-history-head">
                            <span>Past submission</span>
                            <span class="dropbox-history-date">Submitted: ${escapeHtml(submittedAt)}</span>
                        </div>
                        ${renderSubmissionFileLink(submission)}
                        ${note}
                        ${renderStaffSubmissionReadonlyGrade(submission, maxScore)}
                    </div>
                `;
            }).join("")}
        </div>
    `;
}

function togglePastSubmissions(studentKey) {
    const history = document.getElementById(`staff-past-submissions-${studentKey}`);
    const button = document.querySelector(`button[onclick="togglePastSubmissions('${studentKey}')"]`);

    if (!history) {
        return;
    }

    const willShow = history.hidden;
    history.hidden = !willShow;

    if (button) {
        const count = history.querySelectorAll(".staff-past-submission").length;
        button.textContent = willShow
            ? `Hide past submissions (${count})`
            : `View past submissions (${count})`;
    }
}

function renderStaffSubmissionList(submissions, assignment) {
    if (!submissions.length) {
        return '<p class="grades-empty">No student submissions yet.</p>';
    }

    const maxScore = assignment?.max_score;
    const maxScoreLabel = formatGradeNumber(maxScore) || "Not set";
    const submissionGroups = groupSubmissionsByStudent(submissions);

    return `
        <div class="staff-submission-list">
            ${submissionGroups.map(({ latest: submission, past }) => {
                const studentKey = getSubmissionStudentDomKey(submission);
                const submittedAt = formatAssignmentDate(submission.submitted_at);
                const score = submission.score ?? "";
                const feedback = submission.feedback || "";
                const note = submission.submission_text
                    ? `<p class="dropbox-history-note">${escapeHtml(submission.submission_text)}</p>`
                    : "";
                const maxAttribute = Number(maxScore) > 0 ? ` max="${escapeHtml(maxScore)}"` : "";
                const gradeControls = `
                    <div class="staff-grade-form">
                        <label>
                            Score / ${escapeHtml(maxScoreLabel)}
                            <input id="grade-score-${submission.submission_id}" type="number" min="0"${maxAttribute} step="0.01" value="${escapeHtml(score)}">
                        </label>
                        <label>
                            Feedback
                            <textarea id="grade-feedback-${submission.submission_id}" rows="2">${escapeHtml(feedback)}</textarea>
                        </label>
                        <div class="staff-grade-actions">
                            <button type="button" onclick="saveSubmissionGrade(${submission.submission_id})">Save Grade</button>
                            <button type="button" class="danger-btn" onclick="clearSubmissionGrade(${submission.submission_id})">Clear Grade</button>
                        </div>
                    </div>
                `;

                return `
                    <div class="staff-submission-item">
                        <div class="staff-submission-head">
                            <div>
                                <strong>${escapeHtml(submission.student_name || "Student")}</strong>
                                <span>${escapeHtml(submission.student_email || "")}</span>
                            </div>
                            <div class="staff-submission-meta">
                                <span class="dropbox-history-pill">Latest</span>
                                <span class="dropbox-history-pill">Max: ${escapeHtml(maxScoreLabel)}</span>
                                <span class="dropbox-history-date">Submitted: ${escapeHtml(submittedAt)}</span>
                            </div>
                        </div>
                        ${renderSubmissionFileLink(submission)}
                        ${note}
                        ${gradeControls}
                        ${renderStaffPastSubmissions(past, maxScore, studentKey)}
                    </div>
                `;
            }).join("")}
        </div>
    `;
}

async function loadStaffAssignmentSubmissions(assignmentId) {
    const list = document.getElementById("assignment-submissions-list");

    if (!list) {
        return;
    }

    list.innerHTML = '<p class="grades-empty">Loading student submissions...</p>';

    try {
        const response = await axios.get(`/api/assignments/${assignmentId}/submissions`);
        const assignment = currentAssignments.find((item) => Number(item.assignment_id) === Number(assignmentId));
        list.innerHTML = renderStaffSubmissionList(response.data || [], assignment);
    } catch (error) {
        const message = error.response?.data || "Failed to load student submissions.";
        list.innerHTML = `<p class="grades-error">${escapeHtml(message)}</p>`;
    }
}

async function saveSubmissionGrade(submissionId) {
    const scoreInput = document.getElementById(`grade-score-${submissionId}`);
    const feedbackInput = document.getElementById(`grade-feedback-${submissionId}`);

    if (!scoreInput || !feedbackInput) {
        return;
    }

    const score = scoreInput.value.trim();

    if (score === "" || Number(score) < 0) {
        alert("Please enter a score of 0 or higher");
        return;
    }

    const maxScore = Number(scoreInput.max);

    if (Number.isFinite(maxScore) && maxScore >= 0 && Number(score) > maxScore) {
        alert(`Score cannot be greater than ${formatGradeNumber(maxScore)}`);
        return;
    }

    try {
        await axios.put(`/api/submissions/${submissionId}/grade`, {
            score: Number(score),
            feedback: feedbackInput.value.trim() || null,
        });

        if (currentAssignmentDetailsId) {
            await loadStaffAssignmentSubmissions(currentAssignmentDetailsId);
        }

        gradesLoaded = false;
        showActionMessage("Grade saved.", "success");
    } catch (error) {
        showActionMessage(error.response?.data || "Failed to save grade.", "error");
    }
}

async function clearSubmissionGrade(submissionId) {
    if (!confirm("Clear this grade?")) {
        return;
    }

    try {
        await axios.delete(`/api/submissions/${submissionId}/grade`);

        if (currentAssignmentDetailsId) {
            await loadStaffAssignmentSubmissions(currentAssignmentDetailsId);
        }

        gradesLoaded = false;
        showActionMessage("Grade cleared.", "success");
    } catch (error) {
        showActionMessage(error.response?.data || "Failed to clear grade.", "error");
    }
}

function populateDueTimeOptions() {
    const timeSelect = document.getElementById("assignment-due-time-input");

    if (!timeSelect || timeSelect.options.length) {
        return;
    }

    for (let hour = 0; hour < 24; hour += 1) {
        for (const minute of ["00", "30"]) {
            const value = `${String(hour).padStart(2, "0")}:${minute}`;
            timeSelect.innerHTML += `<option value="${value}">${value}</option>`;
        }
    }
}

function setAssignmentSaveState(isSaving, message = "") {
    const saveButton = document.getElementById("save-assignment-btn");
    const closeButton = document.getElementById("close-assignment-modal-btn");
    const status = document.getElementById("assignment-save-status");

    if (saveButton) {
        saveButton.disabled = isSaving;
        saveButton.textContent = isSaving ? "Saving..." : "Save";
    }

    if (closeButton) {
        closeButton.disabled = isSaving;
    }

    if (status) {
        status.textContent = message;
    }
}

function getAssignmentSubmissionLabel(assignment) {
    const methods = [];

    if (assignment.allow_text_submission ?? true) {
        methods.push("Text");
    }

    if (assignment.allow_file_submission ?? true) {
        methods.push("File");
    }

    return methods.length ? methods.join(" and ") : "No submission method set";
}

function getAssignmentPrerequisiteModules(assignment) {
    const prerequisiteIds = Array.isArray(assignment?.prerequisite_module_ids)
        ? assignment.prerequisite_module_ids.map(Number)
        : [];

    return prerequisiteIds
        .map((moduleId) => currentModules.find((item) => Number(item.module_id) === moduleId))
        .filter(Boolean)
        .sort((first, second) => Number(first.position) - Number(second.position));
}

function getAssignmentPrerequisiteLabel(assignment) {
    const modules = getAssignmentPrerequisiteModules(assignment);

    if (!modules.length) {
        return "None";
    }

    return modules
        .map((module) => `${module.position}. ${module.title || "Untitled module"}`)
        .join(", ");
}

function getFirstIncompleteAssignmentPrerequisite(assignment) {
    if (isInstructor || !isEnrolled) {
        return null;
    }

    return getAssignmentPrerequisiteModules(assignment)
        .find((module) => getModuleProgressPercent(module.module_id) < 100) || null;
}

function getAssignmentModuleTitle(assignment) {
    const moduleId = Number(assignment?.module_id);
    const module = currentModules.find((item) => Number(item.module_id) === moduleId);

    return module?.title || "Module not set";
}

function populateAssignmentModuleOptions(selectedModuleId = null) {
    const select = document.getElementById("assignment-module-input");

    if (!select) {
        return;
    }

    if (!currentModules.length) {
        select.innerHTML = '<option value="">Create a module first</option>';
        select.disabled = true;
        return;
    }

    select.disabled = false;
    select.innerHTML = currentModules
        .slice()
        .sort((first, second) => Number(first.position) - Number(second.position))
        .map((module) => `
            <option value="${module.module_id}">${escapeHtml(`M${module.position}: ${module.title}`)}</option>
        `)
        .join("");

    select.value = String(selectedModuleId || currentModules[0].module_id);
}

function populateAssignmentPrerequisiteOptions(assignment = null) {
    const prerequisiteInput = document.getElementById("assignment-prerequisites-input");

    if (!prerequisiteInput) {
        return;
    }

    const selectedIds = new Set(
        Array.isArray(assignment?.prerequisite_module_ids)
            ? assignment.prerequisite_module_ids.map(Number)
            : []
    );

    const options = currentModules
        .slice()
        .sort((first, second) => Number(first.position) - Number(second.position))
        .map((module) => {
            const moduleId = Number(module.module_id);
            const checked = selectedIds.has(moduleId) ? "checked" : "";
            const label = `${module.position}. ${module.title || "Untitled module"}`;

            return `
                <label class="prerequisite-option">
                    <input type="checkbox" value="${moduleId}" ${checked}>
                    <span>${escapeHtml(label)}</span>
                </label>
            `;
        })
        .join("");

    prerequisiteInput.innerHTML = options || '<p class="prerequisite-empty">Create a module first.</p>';
}

function getSelectedAssignmentPrerequisiteIds() {
    const prerequisiteInput = document.getElementById("assignment-prerequisites-input");

    if (!prerequisiteInput) {
        return [];
    }

    return [...prerequisiteInput.querySelectorAll('input[type="checkbox"]:checked')]
        .map((input) => Number(input.value));
}

function getFileTypeLabel(value) {
    const labels = {
        pdf: "PDF",
        docx: "Word document (.docx)",
        pptx: "PowerPoint (.pptx)",
        xlsx: "Excel spreadsheet (.xlsx)",
        zip: "ZIP archive",
        image: "Image",
    };

    return labels[value] || "Any file type";
}

async function uploadAssignmentBrief(file) {
    const formData = new FormData();
    formData.append("file", file);
    formData.append("folder", "lms/assignments");

    const response = await axios.post("/api/cloudinary/upload", formData, {
        headers: {
            "Content-Type": "multipart/form-data",
        },
    });

    return response.data.secure_url;
}

async function uploadSubmissionFile(file) {
    const formData = new FormData();
    formData.append("file", file);
    formData.append("folder", "lms/submissions");

    const response = await axios.post("/api/cloudinary/upload", formData, {
        headers: {
            "Content-Type": "multipart/form-data",
        },
    });

    return response.data;
}

function setDropboxSubmitState(isSaving, message = "") {
    const submitButton = document.getElementById("submit-assignment-dropbox-btn");
    const status = document.getElementById("assignment-dropbox-status");

    if (submitButton) {
        submitButton.disabled = isSaving;
        submitButton.textContent = isSaving ? "Submitting..." : "Submit Assignment";
    }

    if (status) {
        status.textContent = message;
    }
}

async function submitAssignmentDropbox() {
    if (!currentAssignmentDetailsId) {
        return;
    }

    const assignment = currentAssignments.find((item) => item.assignment_id === currentAssignmentDetailsId);
    const fileInput = document.getElementById("assignment-dropbox-file-input");
    const noteInput = document.getElementById("assignment-dropbox-note-input");
    const file = fileInput?.files?.[0];
    const note = noteInput?.value.trim() || null;

    if (!assignment) {
        return;
    }

    const prerequisite = getFirstIncompleteAssignmentPrerequisite(assignment);
    if (prerequisite) {
        setDropboxSubmitState(
            false,
            `Complete ${prerequisite.title || "the prerequisite module"} before submitting this assignment.`
        );
        return;
    }

    if ((assignment.allow_file_submission ?? true) && !file) {
        setDropboxSubmitState(false, "Please choose a file to upload.");
        return;
    }

    const fileError = validateDropboxFile(assignment, file);

    if (fileError) {
        setDropboxSubmitState(false, fileError);
        return;
    }

    try {
        setDropboxSubmitState(true, file ? "Uploading file..." : "Submitting...");

        const upload = file ? await uploadSubmissionFile(file) : null;
        setDropboxSubmitState(true, "Saving submission...");

        await axios.post(`/api/assignments/${currentAssignmentDetailsId}/submissions`, {
            submission_text: note,
            file_url: upload?.secure_url || null,
            cloudinary_public_id: upload?.public_id || null,
            file_name: file?.name || null,
            file_content_type: file?.type || null,
            file_size: file?.size || null,
        });

        gradesLoaded = false;
        setDropboxSubmitState(false, "Assignment submitted.");
        await loadAssignmentSubmissions(currentAssignmentDetailsId);
    } catch (error) {
        if (error.response?.status === 401) {
            window.location.href = "/login";
            return;
        }

        const message = error.response?.data || "Failed to submit assignment.";
        setDropboxSubmitState(false, message);
    }
}

function openAssignmentModal(assignment = null) {
    populateDueTimeOptions();
    populateAssignmentModuleOptions(assignment?.module_id || null);
    populateAssignmentPrerequisiteOptions(assignment);
    setAssignmentSaveState(false, "");
    currentEditingAssignmentId = assignment?.assignment_id || null;
    currentAssignmentBriefUrl = assignment?.assignment_brief_url || null;
    document.getElementById("assignment-modal-title").textContent = assignment ? "Edit Assignment" : "Add Assignment";
    document.getElementById("assignment-title-input").value = assignment?.title || "";
    document.getElementById("assignment-description-input").value = assignment?.description || "";
    document.getElementById("assignment-due-date-input").value = getDateInputValue(assignment?.due_date);
    document.getElementById("assignment-due-time-input").value = getTimeInputValue(assignment?.due_date);
    document.getElementById("assignment-score-input").value = assignment?.max_score ?? "";
    document.getElementById("assignment-passing-mark-input").value = assignment?.passing_mark ?? 50;
    document.getElementById("assignment-brief-file-input").value = "";
    const briefLink = document.getElementById("assignment-brief-current-link");
    if (briefLink) {
        briefLink.href = currentAssignmentBriefUrl || "#";
        briefLink.style.display = currentAssignmentBriefUrl ? "inline-flex" : "none";
    }
    document.getElementById("assignment-file-type-input").value = assignment?.expected_file_type || "";
    document.getElementById("assignment-text-input").checked = assignment?.allow_text_submission ?? true;
    document.getElementById("assignment-file-input").checked = assignment?.allow_file_submission ?? true;
    document.getElementById("assignment-file-size-input").value = assignment?.max_file_size_mb ?? "";
    document.getElementById("assignment-instructions-input").value = assignment?.submission_instructions || "";
    document.getElementById("assignment-modal").style.display = "flex";
}

function closeAssignmentModal() {
    currentEditingAssignmentId = null;
    currentAssignmentBriefUrl = null;
    const prerequisiteInput = document.getElementById("assignment-prerequisites-input");
    if (prerequisiteInput) {
        prerequisiteInput.innerHTML = "";
    }
    document.getElementById("assignment-modal").style.display = "none";
}

function editAssignment(event, assignmentId) {
    event.stopPropagation();
    const assignment = currentAssignments.find((item) => item.assignment_id === assignmentId);

    if (!assignment) {
        return;
    }

    openAssignmentModal(assignment);
}

async function deleteAssignment(event, assignmentId) {
    event.stopPropagation();

    if (!confirm("Delete this assignment?")) {
        return;
    }

    try {
        await axios.delete(`/api/assignment/${assignmentId}`);
        await loadAssignments();
        showActionMessage("Assignment deleted.", "success");
    } catch (error) {
        const message = error.response?.data || "Failed to delete assignment.";
        showActionMessage(message, "error");
    }
}

async function saveAssignment() {
    const title = document.getElementById("assignment-title-input").value.trim();
    const description = document.getElementById("assignment-description-input").value.trim();
    const dueDate = document.getElementById("assignment-due-date-input").value;
    const dueTime = document.getElementById("assignment-due-time-input").value;
    const maxScore = document.getElementById("assignment-score-input").value.trim();
    const moduleId = document.getElementById("assignment-module-input").value;
    const passingMark = document.getElementById("assignment-passing-mark-input").value.trim();
    const briefFile = document.getElementById("assignment-brief-file-input").files[0];
    const expectedFileType = document.getElementById("assignment-file-type-input").value;
    const maxFileSize = document.getElementById("assignment-file-size-input").value.trim();
    const submissionInstructions = document.getElementById("assignment-instructions-input").value.trim();

    if (!title) {
        alert("Please enter an assignment title");
        return;
    }

    if (!description) {
        alert("Please enter an assignment description");
        return;
    }

    if (!dueDate) {
        alert("Please choose a due date");
        return;
    }

    if (!moduleId) {
        alert("Please choose a module");
        return;
    }

    if (maxScore === "" || Number(maxScore) < 0) {
        alert("Please enter a max score of 0 or higher");
        return;
    }

    if (passingMark === "" || Number(passingMark) < 0 || Number(passingMark) > 100) {
        alert("Please enter a passing mark between 0 and 100");
        return;
    }

    try {
        setAssignmentSaveState(true, briefFile ? "Uploading assignment brief..." : "Saving assignment...");

        let briefUrl = currentAssignmentBriefUrl;

        if (briefFile) {
            briefUrl = await uploadAssignmentBrief(briefFile);
            setAssignmentSaveState(true, "Brief uploaded. Saving assignment...");
        }

        const payload = {
            course_id: Number(courseId),
            module_id: Number(moduleId),
            title,
            description,
            due_date: toApiDateTime(`${dueDate}T${dueTime}`),
            max_score: Number(maxScore),
            passing_mark: Number(passingMark),
            assignment_brief_url: briefUrl || null,
            expected_file_type: expectedFileType || null,
            allow_text_submission: document.getElementById("assignment-text-input").checked,
            allow_file_submission: document.getElementById("assignment-file-input").checked,
            max_file_size_mb: maxFileSize ? Number(maxFileSize) : null,
            submission_instructions: submissionInstructions || null,
            prerequisite_module_ids: getSelectedAssignmentPrerequisiteIds(),
        };

        if (currentEditingAssignmentId) {
            await axios.put(`/api/assignment/${currentEditingAssignmentId}`, payload);
        } else {
            await axios.post("/api/assignment", payload);
        }

        closeAssignmentModal();
        await loadAssignments();
        showActionMessage("Assignment saved.", "success");
    } catch (error) {
        const message = error.response?.data || "Failed to save assignment.";
        setAssignmentSaveState(false, "");
        showActionMessage(message, "error");
    }
}
