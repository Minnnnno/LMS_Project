// organisation.js – Organisation view with manual + CSV/Excel mass enrolment

let currentOrgId = null;
let currentOrgName = '';
let allUnassignedUsers = [];       // users with no org yet (for manual tab)
let allSystemUsers = [];           // all users (for file matching)
let selectedEnrollUserIds = new Set();
let fileMatchedRows = [];          // parsed rows from CSV/Excel with matched user_id
let courseInstructorState = { courses: [], instructors: [] };

// ── Helpers ────────────────────────────────────────────────────────────────────

// escHtml, roleBadge, and initials are now provided by lms-core.js / role.js / user.js
function escHtml(str) { return HtmlUtils.escape(str); }
function roleBadge(roleName) { return RoleUtils.badge(roleName); }
function initials(first, last) { return UserUtils.initials(first, last); }

function setProgress(show, label, pct) {
    const el = document.getElementById('upload-progress');
    el.style.display = show ? '' : 'none';
    if (show) {
        document.getElementById('progress-label').textContent = label;
        document.getElementById('progress-pct').textContent = pct + '%';
        document.getElementById('progress-bar').style.width = pct + '%';
    }
}

// ── Tab switching ──────────────────────────────────────────────────────────────

function switchTab(tab) {
    document.getElementById('tab-manual').classList.toggle('active', tab === 'manual');
    document.getElementById('tab-file').classList.toggle('active', tab === 'file');
    document.getElementById('pane-manual').style.display = tab === 'manual' ? '' : 'none';
    document.getElementById('pane-file').style.display   = tab === 'file'   ? '' : 'none';
    document.getElementById('enroll-feedback').innerHTML = '';
}

// ── Organisations list ─────────────────────────────────────────────────────────

async function loadOrganisations() {
    try {
        const { data: orgs } = await axios.get('/api/organisations');
        const el = document.getElementById('org-list');
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
        `).join('');
        document.querySelectorAll('.org-card').forEach(card => {
            const org = orgs.find(item => String(item.org_id) === card.dataset.orgId);
            card.addEventListener('click', () => selectOrg(org.org_id, org.org_name));
        });
        if (orgs.length === 1) {
            await selectOrg(orgs[0].org_id, orgs[0].org_name);
        }
    } catch {
        document.getElementById('org-list').innerHTML =
            '<p class="text-danger small">Failed to load organisations.</p>';
    }
}

// ── Select org & show members ─────────────────────────────────────────────────

async function selectOrg(orgId, orgName) {
    currentOrgId = orgId;
    currentOrgName = orgName;
    document.querySelectorAll('.org-card').forEach(c => c.classList.remove('active'));
    document.getElementById(`org-card-${orgId}`)?.classList.add('active');

    document.getElementById('members-panel').style.display = '';
    document.getElementById('members-title').textContent = 'Members';
    document.getElementById('members-list').innerHTML = '<p class="text-muted small">Loading…</p>';
    hideEnrolPanel();

    try {
        const { data: members } = await axios.get(`/api/organisations/${orgId}/members`);
        renderMembers(members, orgId);
    } catch {
        document.getElementById('members-list').innerHTML =
            '<p class="text-danger small">Failed to load members.</p>';
    }

    await loadCourseInstructorManager(orgId);
}

function renderMembers(members, orgId) {
    const el = document.getElementById('members-list');
    if (!members.length) {
        el.innerHTML = '<p class="text-muted small">No members yet. Use "Add Learners" to add some.</p>';
        return;
    }
    el.innerHTML = members.map(m => `
        <div class="member-row">
            <div class="member-avatar">${initials(m.first_name, m.last_name)}</div>
            <div class="flex-grow-1 min-w-0">
                <div class="fw-semibold">${escHtml(m.first_name)} ${escHtml(m.last_name)}</div>
                <div class="text-muted small text-truncate">${escHtml(m.email)}</div>
            </div>
            <div class="d-flex gap-1 flex-wrap">${(m.roles || []).map(roleBadge).join('')}</div>
            <button class="btn btn-sm btn-outline-danger rounded-3 ms-1"
                    onclick="removeMember(${orgId}, ${m.user_id})" title="Remove from org">
                <i class="bi bi-person-x"></i>
            </button>
        </div>
    `).join('');
}

async function removeMember(orgId, userId) {
    if (!confirm('Remove this member from the organisation?')) return;
    try {
        await axios.delete(`/api/organisations/${orgId}/members/${userId}`);
        selectOrg(orgId, currentOrgName);
    } catch (err) {
        alert('Failed to remove member: ' + (err.response?.data || err.message));
    }
}

// ── Enrol panel open/close ────────────────────────────────────────────────────

async function loadCourseInstructorManager(orgId, options = {}) {
    const panel = document.getElementById('course-instructor-panel');
    const list = document.getElementById('course-instructor-list');
    panel.style.display = '';
    if (!options.keepFeedback) {
        document.getElementById('course-instructor-feedback').innerHTML = '';
    }
    list.innerHTML = '<p class="text-muted small mb-0">Loading course instructors...</p>';

    try {
        const { data } = await axios.get(`/api/organisations/${orgId}/course-instructors`);
        courseInstructorState = data;
        renderCourseInstructorManager();
    } catch (err) {
        list.innerHTML = `<p class="text-danger small mb-0">${escHtml(err.response?.data || 'Failed to load course instructors.')}</p>`;
    }
}

function renderCourseInstructorManager() {
    const courseSelect = document.getElementById('course-instructor-course');
    const instructorSelect = document.getElementById('course-instructor-user');
    const list = document.getElementById('course-instructor-list');
    const assignButton = document.getElementById('btn-assign-course-instructor');
    const courses = courseInstructorState.courses || [];
    const instructors = courseInstructorState.instructors || [];

    courseSelect.innerHTML = courses.length
        ? courses.map(course => `<option value="${course.course_id}">${escHtml(course.name)}</option>`).join('')
        : '<option value="">No courses found</option>';

    instructorSelect.innerHTML = instructors.length
        ? instructors.map(instructor => `<option value="${instructor.user_id}">${escHtml(instructor.first_name)} ${escHtml(instructor.last_name)} (${escHtml(instructor.email)})</option>`).join('')
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
            `).join('')
            : '<span class="text-muted small">No instructors assigned</span>';

        return `
            <div class="course-instructor-card mb-2">
                <div class="fw-semibold mb-2">${escHtml(course.name)}</div>
                <div class="d-flex flex-wrap gap-2">${chips}</div>
            </div>
        `;
    }).join('');

}

document.getElementById('btn-refresh-course-instructors')?.addEventListener('click', () => {
    if (currentOrgId) loadCourseInstructorManager(currentOrgId);
});

document.getElementById('assign-course-instructor-form')?.addEventListener('submit', async (event) => {
    event.preventDefault();
    if (!currentOrgId) return;

    const courseId = Number(document.getElementById('course-instructor-course').value);
    const instructorId = Number(document.getElementById('course-instructor-user').value);
    if (!courseId || !instructorId) return;

    const button = document.getElementById('btn-assign-course-instructor');
    showCourseInstructorFeedback('Assigning instructor...', 'info');
    button.disabled = true;

    try {
        const { data } = await axios.post(
            `/api/organisations/${currentOrgId}/courses/${courseId}/instructors`,
            { instructor_id: instructorId },
        );
        showCourseInstructorFeedback(`<i class="bi bi-check2-circle me-1"></i>${escHtml(data.message || 'Instructor assigned')}`, 'success');
        await loadCourseInstructorManager(currentOrgId, { keepFeedback: true });
    } catch (err) {
        showCourseInstructorFeedback('Error: ' + escHtml(err.response?.data || err.message), 'error');
    } finally {
        button.disabled = false;
    }
});

async function removeCourseInstructor(courseId, instructorId) {
    if (!currentOrgId) return;
    if (!confirm('Remove this instructor from the course?')) return;

    try {
        await axios.delete(`/api/organisations/${currentOrgId}/courses/${courseId}/instructors/${instructorId}`);
        showCourseInstructorFeedback('<i class="bi bi-check2-circle me-1"></i>Instructor removed from course', 'success');
        await loadCourseInstructorManager(currentOrgId, { keepFeedback: true });
    } catch (err) {
        showCourseInstructorFeedback('Error: ' + escHtml(err.response?.data || err.message), 'error');
    }
}

function hideEnrolPanel() {
    document.getElementById('enroll-panel').style.display = 'none';
    selectedEnrollUserIds.clear();
    fileMatchedRows = [];
    document.getElementById('enroll-feedback').innerHTML = '';
    document.getElementById('invite-instructor-feedback').innerHTML = '';
    const inviteEmail = document.getElementById('invite-instructor-email');
    if (inviteEmail) inviteEmail.value = '';
    clearFilePreview();
}

document.getElementById('btn-open-enroll')?.addEventListener('click', async () => {
    if (!currentOrgId) return;
    const panel = document.getElementById('enroll-panel');
    if (panel.style.display !== 'none') { hideEnrolPanel(); return; }

    const orgName = currentOrgName;
    document.getElementById('enroll-org-name').textContent = orgName;
    selectedEnrollUserIds.clear();
    panel.style.display = '';
    switchTab('manual');

    // Load unassigned users for manual tab
    document.getElementById('enroll-user-list').innerHTML =
        '<p class="text-muted small mb-0">Loading…</p>';
    try {
        const { data } = await axios.get('/api/users/unassigned');
        allUnassignedUsers = data;
        renderEnrollUserList(allUnassignedUsers);
    } catch {
        document.getElementById('enroll-user-list').innerHTML =
            '<p class="text-danger small mb-0">Failed to load users.</p>';
    }

    // Load ALL users for file matching (silently)
    try {
        const { data } = await axios.get('/api/users/all');
        allSystemUsers = data;
    } catch {
        // fallback: use unassigned list for matching
        allSystemUsers = allUnassignedUsers;
    }
});

document.getElementById('btn-cancel-enroll')?.addEventListener('click', hideEnrolPanel);

document.getElementById('invite-instructor-form')?.addEventListener('submit', async (event) => {
    event.preventDefault();
    if (!currentOrgId) return;

    const emailInput = document.getElementById('invite-instructor-email');
    const button = document.getElementById('btn-invite-instructor');
    const email = emailInput.value.trim();
    if (!email) return;

    showInviteFeedback('Sending invite...', 'info');
    button.disabled = true;

    try {
        const { data } = await axios.post(`/api/organisations/${currentOrgId}/instructors/invite`, { email });
        showInviteFeedback(`<i class="bi bi-check2-circle me-1"></i>${escHtml(data.message || 'Instructor invited successfully')}`, 'success');
        emailInput.value = '';

        const { data: members } = await axios.get(`/api/organisations/${currentOrgId}/members`);
        renderMembers(members, currentOrgId);
    } catch (err) {
        showInviteFeedback('Error: ' + escHtml(err.response?.data || err.message), 'error');
    } finally {
        button.disabled = false;
    }
});

// ── Manual tab: user list ─────────────────────────────────────────────────────

function renderEnrollUserList(users) {
    const el = document.getElementById('enroll-user-list');
    if (!users.length) {
        el.innerHTML = '<p class="text-muted small mb-0">No unassigned users found.</p>';
        return;
    }
    el.innerHTML = users.map(u => `
        <div class="enroll-user-item">
            <input type="checkbox" class="form-check-input" id="eu-${u.user_id}"
                   value="${u.user_id}"
                   ${selectedEnrollUserIds.has(u.user_id) ? 'checked' : ''}
                   onchange="toggleEnrollUser(${u.user_id}, this.checked)">
            <label class="form-check-label small" for="eu-${u.user_id}">
                <strong>${escHtml(u.first_name)} ${escHtml(u.last_name)}</strong>
                <span class="text-muted"> – ${escHtml(u.email)}</span>
            </label>
        </div>
    `).join('');
}

function toggleEnrollUser(userId, checked) {
    if (checked) selectedEnrollUserIds.add(userId);
    else selectedEnrollUserIds.delete(userId);
}

document.getElementById('enroll-search')?.addEventListener('input', function () {
    const q = this.value.toLowerCase();
    const filtered = allUnassignedUsers.filter(u =>
        `${u.first_name} ${u.last_name} ${u.email}`.toLowerCase().includes(q));
    renderEnrollUserList(filtered);
});

document.getElementById('btn-do-enroll')?.addEventListener('click', () =>
    doEnroll([...selectedEnrollUserIds], 'manual'));

// ── File upload tab ───────────────────────────────────────────────────────────

// Template download
document.getElementById('btn-download-template')?.addEventListener('click', e => {
    e.preventDefault();
    const csv = 'email,first_name,last_name\njohn.doe@example.com,John,Doe\njane.smith@example.com,Jane,Smith\n';
    const blob = new Blob([csv], { type: 'text/csv' });
    const a = Object.assign(document.createElement('a'), {
        href: URL.createObjectURL(blob), download: 'enrolment_template.csv'
    });
    a.click();
});

// Drag & drop
const dropZone = document.getElementById('drop-zone');
dropZone?.addEventListener('dragover', e => { e.preventDefault(); dropZone.classList.add('drag-over'); });
dropZone?.addEventListener('dragleave', () => dropZone.classList.remove('drag-over'));
dropZone?.addEventListener('drop', e => {
    e.preventDefault();
    dropZone.classList.remove('drag-over');
    const file = e.dataTransfer.files[0];
    if (file) handleFile(file);
});

document.getElementById('file-input')?.addEventListener('change', function () {
    if (this.files[0]) handleFile(this.files[0]);
    this.value = ''; // reset so same file can be re-selected
});

function clearFilePreview() {
    document.getElementById('csv-preview-wrap').style.display = 'none';
    document.getElementById('file-enroll-actions').style.display = 'none';
    document.getElementById('csv-preview-body').innerHTML = '';
    document.getElementById('preview-summary').textContent = '';
    setProgress(false);
    fileMatchedRows = [];
}

document.getElementById('btn-clear-file')?.addEventListener('click', clearFilePreview);

function isValidEmail(value) {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

async function handleFile(file) {
    clearFilePreview();
    document.getElementById('enroll-feedback').innerHTML = '';

    const ext = file.name.split('.').pop().toLowerCase();
    if (!['csv', 'xlsx', 'xls'].includes(ext)) {
        showFeedback('Please upload a .csv, .xlsx, or .xls file.', 'error');
        return;
    }

    setProgress(true, 'Reading file…', 10);

    try {
        let rows;
        if (ext === 'csv') {
            rows = await parseCSV(file);
        } else {
            rows = await parseExcel(file);
        }

        setProgress(true, 'Matching against system users…', 60);

        // Normalise headers (lowercase, trim)
        const normalised = rows.map(r => {
            const out = {};
            for (const k of Object.keys(r)) out[k.trim().toLowerCase()] = String(r[k] ?? '').trim();
            return out;
        }).filter(r => r.email);

        if (!normalised.length) {
            setProgress(false);
            showFeedback('No rows with an "email" column found in the file.', 'error');
            return;
        }

        // Build lookup: email → user
        const emailMap = {};
        for (const u of allSystemUsers) emailMap[u.email.toLowerCase()] = u;
        // Also try unassigned as fallback
        for (const u of allUnassignedUsers) emailMap[u.email.toLowerCase()] = u;

        fileMatchedRows = normalised.map(r => {
            const email = r.email.toLowerCase();
            const matched = emailMap[email] || null;
            const firstName = r.first_name || r.firstname || r['first name'] || '';
            const lastName = r.last_name || r.lastname || r['last name'] || '';
            let status = 'create';
            let statusLabel = 'Will create account';

            if (!isValidEmail(email)) {
                status = 'invalid';
                statusLabel = 'Invalid email';
            } else if (matched && (matched.org_id === null || matched.org_id === undefined)) {
                status = 'existing';
                statusLabel = 'Ready to add';
            } else if (matched && Number(matched.org_id) === Number(currentOrgId)) {
                status = 'current';
                statusLabel = 'Already in this organisation';
            } else if (matched) {
                status = 'other_org';
                statusLabel = 'In another organisation';
            }

            return {
                email,
                first_name: firstName,
                last_name: lastName,
                matched,
                status,
                statusLabel,
            };
        });

        setProgress(true, 'Building preview…', 90);
        renderFilePreview(fileMatchedRows);
        setProgress(false);

    } catch (err) {
        setProgress(false);
        showFeedback('Failed to parse file: ' + err.message, 'error');
    }
}

function parseCSV(file) {
    return new Promise((resolve, reject) => {
        Papa.parse(file, {
            header: true,
            skipEmptyLines: true,
            complete: r => resolve(r.data),
            error: e => reject(e),
        });
    });
}

function parseExcel(file) {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = e => {
            try {
                const wb = XLSX.read(e.target.result, { type: 'array' });
                const ws = wb.Sheets[wb.SheetNames[0]];
                resolve(XLSX.utils.sheet_to_json(ws, { defval: '' }));
            } catch (err) { reject(err); }
        };
        reader.onerror = () => reject(new Error('FileReader error'));
        reader.readAsArrayBuffer(file);
    });
}

function renderFilePreview(rows) {
    const existing = rows.filter(r => r.status === 'existing');
    const create = rows.filter(r => r.status === 'create');
    const actionable = existing.length + create.length;
    const skipped = rows.length - actionable;
    const tbody = document.getElementById('csv-preview-body');

    tbody.innerHTML = rows.map((r, i) => {
        const canImport = r.status === 'existing' || r.status === 'create';
        const cls = canImport ? 'preview-match' : 'preview-nomatch';
        const statusClass = canImport ? 'text-success' : 'text-danger';
        const icon = canImport ? 'bi-check-circle-fill' : 'bi-x-circle-fill';
        const status = `<span class="${statusClass}"><i class="bi ${icon}"></i> ${escHtml(r.statusLabel)}</span>`;
        return `<tr class="${cls}">
            <td>${i + 1}</td>
            <td>${escHtml(r.email)}</td>
            <td>${escHtml(r.first_name)}</td>
            <td>${escHtml(r.last_name)}</td>
            <td>${status}</td>
        </tr>`;
    }).join('');

    const matched = { length: actionable };

    document.getElementById('preview-summary').textContent =
        `${rows.length} row(s) found · ${matched.length} matched · ${rows.length - matched.length} will be skipped`;

    document.getElementById('csv-preview-wrap').style.display = '';
    document.getElementById('file-enroll-count').textContent = matched.length;
    document.getElementById('file-enroll-actions').style.display = matched.length ? '' : 'none';
    document.getElementById('preview-summary').textContent =
        `${rows.length} row(s) found - ${existing.length} existing - ${create.length} new - ${skipped} skipped`;
    document.getElementById('file-enroll-count').textContent = actionable;
    document.getElementById('file-enroll-actions').style.display = actionable ? '' : 'none';
}

document.getElementById('btn-do-file-enroll')?.addEventListener('click', () => {
    const ids = fileMatchedRows
        .filter(r => r.status === 'existing' && r.matched)
        .map(r => r.matched.user_id);
    doEnroll(ids, 'file');
});

// ── Shared enrol call ─────────────────────────────────────────────────────────

async function doEnroll(userIds, source, newUsers = []) {
    if (!currentOrgId) return;

    if (source === 'file' && !newUsers.length) {
        newUsers = fileMatchedRows
            .filter(r => r.status === 'create')
            .map(r => ({
                email: r.email,
                first_name: r.first_name,
                last_name: r.last_name,
            }));
    }

    if (!userIds.length && !newUsers.length) {
        showFeedback('No users selected.', 'error');
        return;
    }

    const role = document.getElementById('enroll-role').value;
    showFeedback('Adding learners...', 'info');

    try {
        const { data } = await axios.post(`/api/organisations/${currentOrgId}/enroll`, {
            user_ids: userIds,
            new_users: newUsers,
            role,
        });

        const type = data.errors.length ? 'partial' : 'success';
        let msg = `<i class="bi bi-check2-circle me-1"></i>${escHtml(data.message)}`;
        if (data.errors.length) {
            msg += `<br><small>Skipped: ${data.errors.map(escHtml).join('; ')}</small>`;
        }
        showFeedback(msg, type);

        // Refresh members
        await selectOrg(currentOrgId, currentOrgName);

        // Refresh user lists
        try {
            const { data: fresh } = await axios.get('/api/users/unassigned');
            allUnassignedUsers = fresh;
            if (source === 'manual') renderEnrollUserList(fresh);
        } catch {}
        try {
            const { data: freshAll } = await axios.get('/api/users/all');
            allSystemUsers = freshAll;
        } catch {}

        selectedEnrollUserIds.clear();
        if (source === 'file') clearFilePreview();

    } catch (err) {
        showFeedback('Error: ' + escHtml(err.response?.data || err.message), 'error');
    }
}

function showFeedback(msg, type) {
    const el = document.getElementById('enroll-feedback');
    const cls = type === 'success' ? 'success' : type === 'partial' ? 'partial' : type === 'info' ? '' : 'error';
    el.innerHTML = `<div class="result-summary ${cls}">${msg}</div>`;
}

// ── Boot ──────────────────────────────────────────────────────────────────────
function showInviteFeedback(msg, type) {
    const el = document.getElementById('invite-instructor-feedback');
    const cls = type === 'success' ? 'success' : type === 'info' ? '' : 'error';
    el.innerHTML = `<div class="result-summary ${cls}">${msg}</div>`;
}

function showCourseInstructorFeedback(msg, type) {
    const el = document.getElementById('course-instructor-feedback');
    const cls = type === 'success' ? 'success' : type === 'info' ? '' : 'error';
    el.innerHTML = `<div class="result-summary ${cls}">${msg}</div>`;
}

loadOrganisations();
