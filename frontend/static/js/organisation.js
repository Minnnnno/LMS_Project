// Organisation dashboard: members, course instructors, and learner classes.

let currentOrgId = null;
let currentOrgName = "";
let allSystemUsers = [];
let courseInstructorState = { courses: [], instructors: [] };
let learnerClassState = { classes: [], courses: [] };
let selectedClassId = null;
let editingCourseClassId = null;
let classImportRows = [];

function escHtml(str) { return HtmlUtils.escape(str); }
function roleBadge(roleName) { return RoleUtils.badge(roleName); }
function initials(first, last) { return UserUtils.initials(first, last); }

function showResult(elementId, msg, type) {
    const el = document.getElementById(elementId);
    if (!el) return;
    const cls = type === "success" ? "success" : type === "partial" ? "partial" : type === "info" ? "" : "error";
    el.innerHTML = `<div class="result-summary ${cls}">${msg}</div>`;
}

function validEmail(value) {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

async function loadOrganisations() {
    try {
        const { data: orgs } = await axios.get("/api/organisations");
        const el = document.getElementById("org-list");
        if (!orgs.length) {
            el.innerHTML = '<p class="text-muted small">No organisations yet.</p>';
            return;
        }

        el.innerHTML = orgs.map(org => `
            <div class="org-card mb-2" id="org-card-${org.org_id}" data-org-id="${org.org_id}">
                <div class="d-flex align-items-center justify-content-between">
                    <div class="d-flex align-items-center gap-3">
                        <div class="member-avatar"><i class="bi bi-building"></i></div>
                        <div>
                            <div class="fw-semibold org-card-title">${escHtml(org.org_name)}</div>
                            <div class="text-muted small">Organisation ID ${org.org_id}</div>
                        </div>
                    </div>
                    <i class="bi bi-chevron-right text-muted"></i>
                </div>
            </div>
        `).join("");

        document.querySelectorAll(".org-card").forEach(card => {
            const org = orgs.find(item => String(item.org_id) === card.dataset.orgId);
            card.addEventListener("click", () => selectOrg(org.org_id, org.org_name));
        });

        if (orgs.length === 1) {
            await selectOrg(orgs[0].org_id, orgs[0].org_name);
        }
    } catch {
        document.getElementById("org-list").innerHTML =
            '<p class="text-danger small">Failed to load organisations.</p>';
    }
}

async function selectOrg(orgId, orgName) {
    currentOrgId = orgId;
    currentOrgName = orgName;
    selectedClassId = null;

    document.querySelectorAll(".org-card").forEach(card => card.classList.remove("active"));
    document.getElementById(`org-card-${orgId}`)?.classList.add("active");
    document.getElementById("members-panel").style.display = "";
    document.getElementById("course-instructor-panel").style.display = "";
    document.getElementById("learner-class-panel").style.display = "";
    document.getElementById("members-list").innerHTML = '<p class="text-muted small">Loading...</p>';

    await Promise.all([
        loadUserDirectory(true),
        loadOrganisationMembers(orgId),
        loadCourseInstructorManager(orgId),
        loadLearnerClasses(orgId),
    ]);
}

async function loadUserDirectory(force = false) {
    if (allSystemUsers.length && !force) return;
    try {
        const { data } = await axios.get("/api/users/all");
        allSystemUsers = data;
    } catch {
        allSystemUsers = [];
    }
}

async function loadOrganisationMembers(orgId) {
    try {
        const { data: members } = await axios.get(`/api/organisations/${orgId}/members`);
        renderMembers(members, orgId);
    } catch {
        document.getElementById("members-list").innerHTML =
            '<p class="text-danger small">Failed to load members.</p>';
    }
}

function renderMembers(members, orgId) {
    const el = document.getElementById("members-list");
    if (!members.length) {
        el.innerHTML = '<p class="text-muted small">No members yet. Add learners through Learner Classes below.</p>';
        return;
    }
    el.innerHTML = members.map(m => `
        <div class="member-row">
            <div class="member-avatar">${initials(m.first_name, m.last_name)}</div>
            <div class="flex-grow-1 min-w-0">
                <div class="fw-semibold">${escHtml(m.first_name)} ${escHtml(m.last_name)}</div>
                <div class="text-muted small text-truncate">${escHtml(m.email)}</div>
            </div>
            <div class="d-flex gap-1 flex-wrap">${(m.roles || []).map(roleBadge).join("")}</div>
            <button class="btn btn-sm btn-outline-danger rounded-3 ms-1"
                    onclick="removeMember(${orgId}, ${m.user_id})" title="Remove from org">
                <i class="bi bi-person-x"></i>
            </button>
        </div>
    `).join("");
}

async function removeMember(orgId, userId) {
    if (!confirm("Remove this member from the organisation?")) return;
    try {
        await axios.delete(`/api/organisations/${orgId}/members/${userId}`);
        await selectOrg(orgId, currentOrgName);
    } catch (err) {
        alert("Failed to remove member: " + (err.response?.data || err.message));
    }
}

async function loadCourseInstructorManager(orgId, options = {}) {
    const list = document.getElementById("course-instructor-list");
    if (!options.keepFeedback) document.getElementById("course-instructor-feedback").innerHTML = "";
    list.innerHTML = '<p class="text-muted small mb-0">Loading course instructors...</p>';

    try {
        const { data } = await axios.get(`/api/organisations/${orgId}/course-instructors`);
        courseInstructorState = data;
        renderCourseInstructorManager();
    } catch (err) {
        list.innerHTML = `<p class="text-danger small mb-0">${escHtml(err.response?.data || "Failed to load course instructors.")}</p>`;
    }
}

function renderCourseInstructorManager() {
    const courseSelect = document.getElementById("course-instructor-course");
    const instructorSelect = document.getElementById("course-instructor-user");
    const list = document.getElementById("course-instructor-list");
    const assignButton = document.getElementById("btn-assign-course-instructor");
    const courses = courseInstructorState.courses || [];
    const instructors = courseInstructorState.instructors || [];

    courseSelect.innerHTML = courses.length
        ? courses.map(course => `<option value="${course.course_id}">${escHtml(course.name)}</option>`).join("")
        : '<option value="">No courses found</option>';
    instructorSelect.innerHTML = instructors.length
        ? instructors.map(instructor => `<option value="${instructor.user_id}">${escHtml(instructor.first_name)} ${escHtml(instructor.last_name)} (${escHtml(instructor.email)})</option>`).join("")
        : '<option value="">No instructors found</option>';
    assignButton.disabled = !courses.length || !instructors.length;

    if (!courses.length) {
        list.innerHTML = '<p class="text-muted small mb-0">No courses found in this organisation.</p>';
        return;
    }

    list.innerHTML = courses.map(course => {
        const chips = (course.instructors || []).length
            ? course.instructors.map(instructor => `
                <span class="course-instructor-chip">
                    ${escHtml(instructor.first_name)} ${escHtml(instructor.last_name)}
                    <button type="button" title="Remove instructor"
                            onclick="removeCourseInstructor(${course.course_id}, ${instructor.user_id})">
                        <i class="bi bi-x-circle"></i>
                    </button>
                </span>
            `).join("")
            : '<span class="text-muted small">No instructors assigned</span>';

        return `
            <div class="course-instructor-card mb-2">
                <div class="fw-semibold mb-2">${escHtml(course.name)}</div>
                <div class="d-flex flex-wrap gap-2">${chips}</div>
            </div>
        `;
    }).join("");
}

async function loadLearnerClasses(orgId, options = {}) {
    if (!options.keepFeedback) document.getElementById("class-feedback").innerHTML = "";
    document.getElementById("learner-class-list").innerHTML =
        '<div class="col-12"><p class="text-muted small mb-0">Loading classes...</p></div>';

    try {
        const { data } = await axios.get(`/api/organisations/${orgId}/classes`);
        learnerClassState = data;
        if (!selectedClassId && data.classes.length) selectedClassId = data.classes[0].class_id;
        if (selectedClassId && !data.classes.some(cls => cls.class_id === selectedClassId)) {
            selectedClassId = data.classes[0]?.class_id || null;
        }
        renderLearnerClassManager();
    } catch (err) {
        document.getElementById("learner-class-list").innerHTML =
            `<div class="col-12"><p class="text-danger small mb-0">${escHtml(err.response?.data || "Failed to load classes.")}</p></div>`;
    }
}

function renderLearnerClassManager() {
    const courses = learnerClassState.courses || [];
    const classes = learnerClassState.classes || [];
    const classList = document.getElementById("learner-class-list");

    renderCourseCheckboxList("class-course-list", [], "class-course");
    document.getElementById("btn-create-class").disabled = !courses.length;

    if (!classes.length) {
        selectedClassId = null;
        classList.innerHTML = '<div class="col-12"><p class="text-muted small mb-0">No learner classes yet. Create one above.</p></div>';
        document.getElementById("class-roster-tools").style.display = "none";
        return;
    }

    classList.innerHTML = classes.map(cls => {
        const members = cls.members || [];
        const classCourses = cls.courses || [];
        const courseNames = classCourses.length
            ? classCourses.map(course => course.name).join(", ")
            : "No courses assigned";
        const editingCourses = cls.class_id === editingCourseClassId;
        const chips = members.length
            ? members.map(member => `
                <span class="class-member-chip">
                    ${escHtml(member.first_name)} ${escHtml(member.last_name)}
                    <button type="button" title="Remove learner"
                            onclick="removeClassMember(${cls.class_id}, ${member.user_id})">
                        <i class="bi bi-x-circle"></i>
                    </button>
                </span>
            `).join("")
            : '<span class="text-muted small">No learners assigned</span>';
        const active = cls.class_id === selectedClassId ? " active" : "";
        return `
            <div class="col-12 col-xl-6">
                <div class="class-manager-card${active}" onclick="selectLearnerClass(${cls.class_id})">
                    <div class="d-flex justify-content-between align-items-start gap-2 mb-2">
                        <div>
                            <div class="fw-semibold">${escHtml(cls.class_name)}</div>
                            <div class="text-muted small">${escHtml(courseNames)} - ${members.length} learner${members.length === 1 ? "" : "s"}</div>
                        </div>
                        <div class="btn-group btn-group-sm" onclick="event.stopPropagation()">
                            <button class="btn btn-outline-secondary" title="Rename" onclick="renameClass(${cls.class_id})">
                                <i class="bi bi-pencil"></i>
                            </button>
                            <button class="btn btn-outline-secondary" title="Change courses" onclick="changeClassCourses(${cls.class_id})">
                                <i class="bi bi-journal-arrow-up"></i>
                            </button>
                            <button class="btn btn-outline-danger" title="Delete" onclick="deleteClass(${cls.class_id})">
                                <i class="bi bi-trash"></i>
                            </button>
                        </div>
                    </div>
                    <div class="d-flex flex-wrap gap-2">${chips}</div>
                    ${editingCourses ? renderClassCourseEditor(cls) : ""}
                </div>
            </div>
        `;
    }).join("");

    renderSelectedClassTools();
}

function renderCourseCheckboxList(containerId, selectedIds = [], inputName = "class-course") {
    const container = document.getElementById(containerId);
    if (!container) return;

    const courses = learnerClassState.courses || [];
    const selected = new Set(selectedIds.map(Number));

    if (!courses.length) {
        container.innerHTML = '<p class="text-muted small mb-0">No courses found in this organisation.</p>';
        return;
    }

    container.innerHTML = courses.map(course => {
        const inputId = `${inputName}-${course.course_id}`;
        return `
            <label class="class-course-option" for="${inputId}">
                <input
                    class="form-check-input m-0"
                    type="checkbox"
                    id="${inputId}"
                    name="${inputName}"
                    value="${course.course_id}"
                    ${selected.has(Number(course.course_id)) ? "checked" : ""}
                >
                <span>${escHtml(course.name)}</span>
            </label>
        `;
    }).join("");
}

function renderClassCourseEditor(cls) {
    const selectedIds = (cls.courses || []).map(course => course.course_id);
    const inputName = `edit-class-course-${cls.class_id}`;
    const courses = learnerClassState.courses || [];
    const options = courses.length
        ? courses.map(course => {
            const inputId = `${inputName}-${course.course_id}`;
            const checked = selectedIds.includes(course.course_id) ? "checked" : "";
            return `
                <label class="class-course-option" for="${inputId}">
                    <input class="form-check-input m-0" type="checkbox" id="${inputId}" name="${inputName}" value="${course.course_id}" ${checked}>
                    <span>${escHtml(course.name)}</span>
                </label>
            `;
        }).join("")
        : '<p class="text-muted small mb-0">No courses found in this organisation.</p>';

    return `
        <div class="class-course-edit-panel" onclick="event.stopPropagation()">
            <label class="form-label fw-semibold small mb-2">Courses in this class</label>
            <div class="class-course-picker">${options}</div>
            <div class="d-flex flex-wrap gap-2 mt-2">
                <button class="btn btn-dark btn-sm rounded-3" type="button" onclick="saveClassCourses(${cls.class_id})">
                    <i class="bi bi-check2 me-1"></i>Save courses
                </button>
                <button class="btn btn-outline-secondary btn-sm rounded-3" type="button" onclick="setEditCourseChecks(${cls.class_id}, true)">
                    Select all
                </button>
                <button class="btn btn-outline-secondary btn-sm rounded-3" type="button" onclick="setEditCourseChecks(${cls.class_id}, false)">
                    Clear
                </button>
                <button class="btn btn-link btn-sm text-secondary" type="button" onclick="cancelClassCourseEdit()">
                    Cancel
                </button>
            </div>
        </div>
    `;
}

function selectedClass() {
    return (learnerClassState.classes || []).find(cls => cls.class_id === selectedClassId) || null;
}

function selectLearnerClass(classId) {
    selectedClassId = classId;
    clearClassImportPreview();
    renderLearnerClassManager();
}

function renderSelectedClassTools() {
    const cls = selectedClass();
    const tools = document.getElementById("class-roster-tools");
    if (!cls) {
        tools.style.display = "none";
        return;
    }

    tools.style.display = "";
    document.getElementById("selected-class-title").textContent = `${cls.class_name} Roster`;
    renderExistingLearnerOptions();
}

function renderExistingLearnerOptions() {
    const cls = selectedClass();
    const select = document.getElementById("class-member-user-select");
    if (!cls) return;

    const currentIds = new Set((cls.members || []).map(member => member.user_id));
    const candidates = allSystemUsers
        .filter(user => !currentIds.has(user.user_id))
        .filter(user => user.org_id == null || Number(user.org_id) === Number(currentOrgId))
        .sort((a, b) => `${a.last_name} ${a.first_name}`.localeCompare(`${b.last_name} ${b.first_name}`));

    select.innerHTML = candidates.length
        ? candidates.map(user => `<option value="${user.user_id}">${escHtml(user.first_name)} ${escHtml(user.last_name)} (${escHtml(user.email)})</option>`).join("")
        : '<option value="">No eligible learners found</option>';
    document.getElementById("btn-add-existing-to-class").disabled = !candidates.length;
}

async function createClass(event) {
    event.preventDefault();
    if (!currentOrgId) return;

    const className = document.getElementById("class-name-input").value.trim();
    const courseIds = selectedClassCourseIds();
    if (!className || !courseIds.length) return;

    try {
        await axios.post(`/api/organisations/${currentOrgId}/classes`, {
            class_name: className,
            course_ids: courseIds,
        });
        document.getElementById("class-name-input").value = "";
        showResult("class-feedback", '<i class="bi bi-check2-circle me-1"></i>Class created', "success");
        await loadLearnerClasses(currentOrgId, { keepFeedback: true });
    } catch (err) {
        showResult("class-feedback", "Error: " + escHtml(err.response?.data || err.message), "error");
    }
}

async function renameClass(classId) {
    const cls = (learnerClassState.classes || []).find(item => item.class_id === classId);
    if (!cls) return;
    const className = prompt("Class name", cls.class_name);
    if (!className || !className.trim()) return;

    try {
        await axios.put(`/api/organisations/${currentOrgId}/classes/${classId}`, {
            class_name: className.trim(),
        });
        showResult("class-feedback", '<i class="bi bi-check2-circle me-1"></i>Class renamed', "success");
        await loadLearnerClasses(currentOrgId, { keepFeedback: true });
    } catch (err) {
        showResult("class-feedback", "Error: " + escHtml(err.response?.data || err.message), "error");
    }
}

function selectedClassCourseIds() {
    return [...document.querySelectorAll('input[name="class-course"]:checked')]
        .map(input => Number(input.value))
        .filter(Boolean);
}

function changeClassCourses(classId) {
    const cls = (learnerClassState.classes || []).find(item => item.class_id === classId);
    if (!cls) return;
    editingCourseClassId = editingCourseClassId === classId ? null : classId;
    renderLearnerClassManager();
}

function editClassCourseIds(classId) {
    return [...document.querySelectorAll(`input[name="edit-class-course-${classId}"]:checked`)]
        .map(input => Number(input.value))
        .filter(Boolean);
}

function setEditCourseChecks(classId, checked) {
    document
        .querySelectorAll(`input[name="edit-class-course-${classId}"]`)
        .forEach(input => { input.checked = checked; });
}

function setCreateCourseChecks(checked) {
    document
        .querySelectorAll('input[name="class-course"]')
        .forEach(input => { input.checked = checked; });
}

function cancelClassCourseEdit() {
    editingCourseClassId = null;
    renderLearnerClassManager();
}

async function saveClassCourses(classId) {
    const courseIds = editClassCourseIds(classId);
    if (!courseIds.length) return;

    try {
        await axios.put(`/api/organisations/${currentOrgId}/classes/${classId}`, {
            course_ids: courseIds,
        });
        showResult("class-feedback", '<i class="bi bi-check2-circle me-1"></i>Class courses updated', "success");
        editingCourseClassId = null;
        await loadLearnerClasses(currentOrgId, { keepFeedback: true });
    } catch (err) {
        showResult("class-feedback", "Error: " + escHtml(err.response?.data || err.message), "error");
    }
}

async function deleteClass(classId) {
    if (!confirm("Delete this class? Learners will lose access to its course unless another class still assigns the same course.")) return;

    try {
        await axios.delete(`/api/organisations/${currentOrgId}/classes/${classId}`);
        showResult("class-feedback", '<i class="bi bi-check2-circle me-1"></i>Class deleted', "success");
        await loadLearnerClasses(currentOrgId, { keepFeedback: true });
        await loadOrganisationMembers(currentOrgId);
    } catch (err) {
        showResult("class-feedback", "Error: " + escHtml(err.response?.data || err.message), "error");
    }
}

async function addExistingLearnerToClass() {
    const cls = selectedClass();
    const userId = Number(document.getElementById("class-member-user-select").value);
    if (!cls || !userId) return;

    await addLearnersToClass(cls.class_id, { user_ids: [userId], new_users: [] });
}

async function addNewLearnerToClass(event) {
    event.preventDefault();
    const cls = selectedClass();
    if (!cls) return;

    const email = document.getElementById("class-new-email").value.trim();
    if (!email) return;

    await addLearnersToClass(cls.class_id, {
        user_ids: [],
        new_users: [{
            email,
            first_name: document.getElementById("class-new-first-name").value.trim(),
            last_name: document.getElementById("class-new-last-name").value.trim(),
        }],
    });

    document.getElementById("class-new-learner-form").reset();
}

async function addLearnersToClass(classId, payload) {
    try {
        const { data } = await axios.post(
            `/api/organisations/${currentOrgId}/classes/${classId}/members`,
            payload,
        );
        showClassMutationResult(data);
        await Promise.all([
            loadUserDirectory(true),
            loadOrganisationMembers(currentOrgId),
            loadLearnerClasses(currentOrgId, { keepFeedback: true }),
        ]);
    } catch (err) {
        showResult("class-feedback", "Error: " + escHtml(err.response?.data || err.message), "error");
    }
}

function showClassMutationResult(data) {
    const type = data.errors?.length ? "partial" : "success";
    let msg = `<i class="bi bi-check2-circle me-1"></i>${escHtml(data.message || "Class updated")}`;
    if (data.errors?.length) {
        msg += `<br><small>Skipped: ${data.errors.map(escHtml).join("; ")}</small>`;
    }
    showResult("class-feedback", msg, type);
}

async function removeClassMember(classId, userId) {
    if (!confirm("Remove this learner from the class?")) return;

    try {
        await axios.delete(`/api/organisations/${currentOrgId}/classes/${classId}/members/${userId}`);
        showResult("class-feedback", '<i class="bi bi-check2-circle me-1"></i>Learner removed from class', "success");
        await loadLearnerClasses(currentOrgId, { keepFeedback: true });
    } catch (err) {
        showResult("class-feedback", "Error: " + escHtml(err.response?.data || err.message), "error");
    }
}

function clearClassImportPreview() {
    classImportRows = [];
    document.getElementById("class-import-preview").style.display = "none";
    document.getElementById("class-preview-body").innerHTML = "";
    document.getElementById("class-preview-summary").textContent = "";
    document.getElementById("class-import-count").textContent = "0";
}

async function handleClassFile(file) {
    clearClassImportPreview();
    const ext = file.name.split(".").pop().toLowerCase();
    if (!["csv", "xlsx", "xls"].includes(ext)) {
        showResult("class-feedback", "Please upload a .csv, .xlsx, or .xls file.", "error");
        return;
    }

    try {
        const rows = ext === "csv" ? await parseCSV(file) : await parseExcel(file);
        const normalized = rows.map(row => {
            const out = {};
            for (const key of Object.keys(row)) out[key.trim().toLowerCase()] = String(row[key] ?? "").trim();
            return out;
        }).filter(row => row.email || row.class_name || row.classname || row["class name"]);

        const classNames = new Set((learnerClassState.classes || []).map(cls => cls.class_name.trim().toLowerCase()));
        const usersByEmail = {};
        for (const user of allSystemUsers) usersByEmail[user.email.toLowerCase()] = user;

        classImportRows = normalized.map(row => {
            const email = (row.email || "").toLowerCase();
            const className = row.class_name || row.classname || row["class name"] || "";
            const firstName = row.first_name || row.firstname || row["first name"] || "";
            const lastName = row.last_name || row.lastname || row["last name"] || "";
            const matched = usersByEmail[email] || null;
            let status = "ready";
            let statusLabel = "Ready";

            if (!validEmail(email)) {
                status = "invalid";
                statusLabel = "Invalid email";
            } else if (!className.trim() || !classNames.has(className.trim().toLowerCase())) {
                status = "unknown_class";
                statusLabel = "Unknown class";
            } else if (matched && matched.org_id != null && Number(matched.org_id) !== Number(currentOrgId)) {
                status = "other_org";
                statusLabel = "In another organisation";
            } else if (matched && Number(matched.org_id) === Number(currentOrgId)) {
                statusLabel = "Existing organisation learner";
            } else if (matched) {
                statusLabel = "Existing unassigned learner";
            } else {
                statusLabel = "Will create account";
            }

            return { email, first_name: firstName, last_name: lastName, class_name: className, status, statusLabel };
        });

        renderClassImportPreview();
    } catch (err) {
        showResult("class-feedback", "Failed to parse file: " + escHtml(err.message), "error");
    }
}

function renderClassImportPreview() {
    const actionable = classImportRows.filter(row => row.status === "ready");
    const body = document.getElementById("class-preview-body");
    body.innerHTML = classImportRows.map((row, index) => {
        const ok = row.status === "ready";
        const statusClass = ok ? "text-success" : "text-danger";
        const icon = ok ? "bi-check-circle-fill" : "bi-x-circle-fill";
        return `
            <tr class="${ok ? "preview-match" : "preview-nomatch"}">
                <td>${index + 1}</td>
                <td>${escHtml(row.email)}</td>
                <td>${escHtml(row.first_name)}</td>
                <td>${escHtml(row.last_name)}</td>
                <td>${escHtml(row.class_name)}</td>
                <td><span class="${statusClass}"><i class="bi ${icon}"></i> ${escHtml(row.statusLabel)}</span></td>
            </tr>
        `;
    }).join("");

    document.getElementById("class-preview-summary").textContent =
        `${classImportRows.length} row(s) found - ${actionable.length} ready - ${classImportRows.length - actionable.length} skipped`;
    document.getElementById("class-import-count").textContent = actionable.length;
    document.getElementById("btn-import-classes").disabled = !actionable.length;
    document.getElementById("class-import-preview").style.display = "";
}

async function importClassRows() {
    const rows = classImportRows
        .filter(row => row.status === "ready")
        .map(row => ({
            email: row.email,
            first_name: row.first_name,
            last_name: row.last_name,
            class_name: row.class_name,
        }));

    if (!rows.length) {
        showResult("class-feedback", "No import rows are ready.", "error");
        return;
    }

    try {
        const { data } = await axios.post(`/api/organisations/${currentOrgId}/classes/import`, { rows });
        clearClassImportPreview();
        showClassMutationResult(data);
        await Promise.all([
            loadUserDirectory(true),
            loadOrganisationMembers(currentOrgId),
            loadLearnerClasses(currentOrgId, { keepFeedback: true }),
        ]);
    } catch (err) {
        showResult("class-feedback", "Error: " + escHtml(err.response?.data || err.message), "error");
    }
}

function parseCSV(file) {
    return new Promise((resolve, reject) => {
        Papa.parse(file, {
            header: true,
            skipEmptyLines: true,
            complete: result => resolve(result.data),
            error: error => reject(error),
        });
    });
}

function parseExcel(file) {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = event => {
            try {
                const workbook = XLSX.read(event.target.result, { type: "array" });
                const worksheet = workbook.Sheets[workbook.SheetNames[0]];
                resolve(XLSX.utils.sheet_to_json(worksheet, { defval: "" }));
            } catch (err) {
                reject(err);
            }
        };
        reader.onerror = () => reject(new Error("FileReader error"));
        reader.readAsArrayBuffer(file);
    });
}

document.getElementById("btn-refresh-course-instructors")?.addEventListener("click", () => {
    if (currentOrgId) loadCourseInstructorManager(currentOrgId);
});

document.getElementById("assign-course-instructor-form")?.addEventListener("submit", async event => {
    event.preventDefault();
    if (!currentOrgId) return;

    const courseId = Number(document.getElementById("course-instructor-course").value);
    const instructorId = Number(document.getElementById("course-instructor-user").value);
    if (!courseId || !instructorId) return;

    const button = document.getElementById("btn-assign-course-instructor");
    showResult("course-instructor-feedback", "Assigning instructor...", "info");
    button.disabled = true;

    try {
        const { data } = await axios.post(
            `/api/organisations/${currentOrgId}/courses/${courseId}/instructors`,
            { instructor_id: instructorId },
        );
        showResult("course-instructor-feedback", `<i class="bi bi-check2-circle me-1"></i>${escHtml(data.message || "Instructor assigned")}`, "success");
        await loadCourseInstructorManager(currentOrgId, { keepFeedback: true });
    } catch (err) {
        showResult("course-instructor-feedback", "Error: " + escHtml(err.response?.data || err.message), "error");
    } finally {
        button.disabled = false;
    }
});

async function removeCourseInstructor(courseId, instructorId) {
    if (!currentOrgId) return;
    if (!confirm("Remove this instructor from the course?")) return;

    try {
        await axios.delete(`/api/organisations/${currentOrgId}/courses/${courseId}/instructors/${instructorId}`);
        showResult("course-instructor-feedback", '<i class="bi bi-check2-circle me-1"></i>Instructor removed from course', "success");
        await loadCourseInstructorManager(currentOrgId, { keepFeedback: true });
    } catch (err) {
        showResult("course-instructor-feedback", "Error: " + escHtml(err.response?.data || err.message), "error");
    }
}

document.getElementById("invite-instructor-form")?.addEventListener("submit", async event => {
    event.preventDefault();
    if (!currentOrgId) return;

    const emailInput = document.getElementById("invite-instructor-email");
    const button = document.getElementById("btn-invite-instructor");
    const email = emailInput.value.trim();
    if (!email) return;

    showResult("invite-instructor-feedback", "Sending invite...", "info");
    button.disabled = true;

    try {
        const { data } = await axios.post(`/api/organisations/${currentOrgId}/instructors/invite`, { email });
        showResult("invite-instructor-feedback", `<i class="bi bi-check2-circle me-1"></i>${escHtml(data.message || "Instructor invited successfully")}`, "success");
        emailInput.value = "";
        await Promise.all([
            loadOrganisationMembers(currentOrgId),
            loadCourseInstructorManager(currentOrgId),
            loadUserDirectory(true),
        ]);
    } catch (err) {
        showResult("invite-instructor-feedback", "Error: " + escHtml(err.response?.data || err.message), "error");
    } finally {
        button.disabled = false;
    }
});

document.getElementById("create-class-form")?.addEventListener("submit", createClass);
document.getElementById("btn-select-all-class-courses")?.addEventListener("click", () => setCreateCourseChecks(true));
document.getElementById("btn-clear-class-courses")?.addEventListener("click", () => setCreateCourseChecks(false));
document.getElementById("btn-refresh-classes")?.addEventListener("click", () => {
    if (currentOrgId) loadLearnerClasses(currentOrgId);
});
document.getElementById("btn-add-existing-to-class")?.addEventListener("click", addExistingLearnerToClass);
document.getElementById("class-new-learner-form")?.addEventListener("submit", addNewLearnerToClass);
document.getElementById("btn-clear-class-file")?.addEventListener("click", clearClassImportPreview);
document.getElementById("btn-import-classes")?.addEventListener("click", importClassRows);

document.getElementById("btn-download-class-template")?.addEventListener("click", event => {
    event.preventDefault();
    const csv = "email,first_name,last_name,class_name\njohn.doe@example.com,John,Doe,Class A\njane.smith@example.com,Jane,Smith,Class B\n";
    const blob = new Blob([csv], { type: "text/csv" });
    const link = Object.assign(document.createElement("a"), {
        href: URL.createObjectURL(blob),
        download: "class_enrolment_template.csv",
    });
    link.click();
});

const classDropZone = document.getElementById("class-drop-zone");
classDropZone?.addEventListener("dragover", event => {
    event.preventDefault();
    classDropZone.classList.add("drag-over");
});
classDropZone?.addEventListener("dragleave", () => classDropZone.classList.remove("drag-over"));
classDropZone?.addEventListener("drop", event => {
    event.preventDefault();
    classDropZone.classList.remove("drag-over");
    const file = event.dataTransfer.files[0];
    if (file) handleClassFile(file);
});
document.getElementById("class-file-input")?.addEventListener("change", function () {
    if (this.files[0]) handleClassFile(this.files[0]);
    this.value = "";
});

Promise.allSettled([
    loadOrganisations(),
    loadUserDirectory(),
]);
