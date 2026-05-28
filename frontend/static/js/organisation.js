// organisation.js – Organisation view: list orgs, view members, mass enrol

let currentOrgId = null;
let allUnassignedUsers = [];
let selectedEnrollUserIds = new Set();

// ── Utilities ──────────────────────────────────────────────────────────────────

function roleBadge(roleName) {
    if (!roleName) return '';
    const cls =
        roleName.includes('Admin')      ? 'badge-admin' :
        roleName.includes('Instructor') ? 'badge-instructor' :
                                          'badge-student';
    return `<span class="badge-role ${cls}">${roleName}</span>`;
}

function initials(first, last) {
    return ((first || '')[0] || '') + ((last || '')[0] || '').toUpperCase();
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
            <div class="org-card mb-2" id="org-card-${org.org_id}" onclick="selectOrg(${org.org_id}, '${escHtml(org.org_name)}')">
                <div class="d-flex align-items-center justify-content-between">
                    <div>
                        <div class="fw-semibold">${escHtml(org.org_name)}</div>
                        <div class="text-muted small">ID ${org.org_id}</div>
                    </div>
                    <i class="bi bi-chevron-right text-muted"></i>
                </div>
            </div>
        `).join('');
    } catch (err) {
        document.getElementById('org-list').innerHTML =
            '<p class="text-danger small">Failed to load organisations.</p>';
    }
}

function escHtml(str) {
    return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}

// ── Select an organisation & load members ──────────────────────────────────────

async function selectOrg(orgId, orgName) {
    currentOrgId = orgId;

    document.querySelectorAll('.org-card').forEach(c => c.classList.remove('active'));
    const card = document.getElementById(`org-card-${orgId}`);
    if (card) card.classList.add('active');

    document.getElementById('members-panel').style.display = '';
    document.getElementById('members-title').textContent = `Members – ${orgName}`;
    document.getElementById('members-list').innerHTML = '<p class="text-muted small">Loading…</p>';

    // Hide enrol panel when switching orgs
    hideEnrolPanel();

    try {
        const { data: members } = await axios.get(`/api/organisations/${orgId}/members`);
        renderMembers(members, orgId);
    } catch (err) {
        document.getElementById('members-list').innerHTML =
            '<p class="text-danger small">Failed to load members.</p>';
    }
}

function renderMembers(members, orgId) {
    const el = document.getElementById('members-list');
    if (!members.length) {
        el.innerHTML = '<p class="text-muted small">No members yet. Use "Enrol Members" to add some.</p>';
        return;
    }

    el.innerHTML = members.map(m => `
        <div class="member-row">
            <div class="member-avatar">${initials(m.first_name, m.last_name)}</div>
            <div class="flex-grow-1">
                <div class="fw-semibold">${escHtml(m.first_name)} ${escHtml(m.last_name)}</div>
                <div class="text-muted small">${escHtml(m.email)}</div>
            </div>
            <div class="d-flex gap-1 flex-wrap">
                ${(m.roles || []).map(roleBadge).join(' ')}
            </div>
            <button class="btn btn-sm btn-outline-danger rounded-3 ms-1"
                    onclick="removeMember(${orgId}, ${m.user_id})"
                    title="Remove from org">
                <i class="bi bi-person-x"></i>
            </button>
        </div>
    `).join('');
}

async function removeMember(orgId, userId) {
    if (!confirm('Remove this member from the organisation?')) return;
    try {
        await axios.delete(`/api/organisations/${orgId}/members/${userId}`);
        selectOrg(orgId, document.getElementById('members-title').textContent.replace('Members – ', ''));
    } catch (err) {
        alert('Failed to remove member: ' + (err.response?.data || err.message));
    }
}

// ── Mass Enrol panel ───────────────────────────────────────────────────────────

function hideEnrolPanel() {
    document.getElementById('enroll-panel').style.display = 'none';
    selectedEnrollUserIds.clear();
    document.getElementById('enroll-feedback').innerHTML = '';
}

document.getElementById('btn-open-enroll')?.addEventListener('click', async () => {
    if (!currentOrgId) return;

    const panel = document.getElementById('enroll-panel');
    if (panel.style.display !== 'none') { hideEnrolPanel(); return; }

    const orgName = document.getElementById('members-title').textContent.replace('Members – ', '');
    document.getElementById('enroll-org-name').textContent = orgName;
    selectedEnrollUserIds.clear();

    panel.style.display = '';
    document.getElementById('enroll-user-list').innerHTML = '<p class="text-muted small">Loading…</p>';

    try {
        const { data } = await axios.get('/api/users/unassigned');
        allUnassignedUsers = data;
        renderEnrollUserList(allUnassignedUsers);
    } catch (err) {
        document.getElementById('enroll-user-list').innerHTML =
            '<p class="text-danger small">Failed to load users.</p>';
    }
});

document.getElementById('btn-cancel-enroll')?.addEventListener('click', hideEnrolPanel);

function renderEnrollUserList(users) {
    const el = document.getElementById('enroll-user-list');
    if (!users.length) {
        el.innerHTML = '<p class="text-muted small mb-0">No unassigned users found.</p>';
        return;
    }
    el.innerHTML = users.map(u => `
        <div class="enroll-user-item">
            <input type="checkbox" class="form-check-input enroll-check" id="eu-${u.user_id}"
                   value="${u.user_id}" ${selectedEnrollUserIds.has(u.user_id) ? 'checked' : ''}
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
        `${u.first_name} ${u.last_name} ${u.email}`.toLowerCase().includes(q)
    );
    renderEnrollUserList(filtered);
});

document.getElementById('btn-do-enroll')?.addEventListener('click', async () => {
    if (!currentOrgId) return;
    if (!selectedEnrollUserIds.size) {
        document.getElementById('enroll-feedback').innerHTML =
            '<p class="text-warning small">Select at least one user.</p>';
        return;
    }

    const role = document.getElementById('enroll-role').value;
    const fb = document.getElementById('enroll-feedback');
    fb.innerHTML = '<p class="text-muted small">Enrolling…</p>';

    try {
        const { data } = await axios.post(`/api/organisations/${currentOrgId}/enroll`, {
            user_ids: [...selectedEnrollUserIds],
            role,
        });

        fb.innerHTML = `<p class="text-success small">${escHtml(data.message)}</p>` +
            (data.errors.length
                ? `<p class="text-danger small">Errors: ${data.errors.map(escHtml).join('; ')}</p>`
                : '');

        // Refresh members list
        const orgName = document.getElementById('members-title').textContent.replace('Members – ', '');
        await selectOrg(currentOrgId, orgName);
        selectedEnrollUserIds.clear();

        // Reload unassigned list
        try {
            const { data: fresh } = await axios.get('/api/users/unassigned');
            allUnassignedUsers = fresh;
            renderEnrollUserList(fresh);
        } catch (_) {}
    } catch (err) {
        fb.innerHTML = `<p class="text-danger small">Error: ${escHtml(err.response?.data || err.message)}</p>`;
    }
});

// ── Create Organisation modal ─────────────────────────────────────────────────

document.getElementById('btn-create-org')?.addEventListener('click', async () => {
    const name = document.getElementById('new-org-name').value.trim();
    const errEl = document.getElementById('create-org-error');

    if (!name) { errEl.textContent = 'Please enter an organisation name.'; return; }
    errEl.textContent = '';

    try {
        await axios.post('/api/organisations', { org_name: name });
        document.getElementById('new-org-name').value = '';
        bootstrap.Modal.getInstance(document.getElementById('createOrgModal')).hide();
        await loadOrganisations();
    } catch (err) {
        errEl.textContent = err.response?.data || 'Failed to create organisation.';
    }
});

// ── Bootstrap ─────────────────────────────────────────────────────────────────
loadOrganisations();
