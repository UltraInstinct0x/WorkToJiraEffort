// Tauri API
const { invoke } = window.__TAURI__.core;

// DOM Elements
const currentIssueEl = document.getElementById('currentIssue');
const trackingStatusEl = document.getElementById('trackingStatus');
const totalTimeEl = document.getElementById('totalTime');
const daemonStatusEl = document.getElementById('daemonStatus');
const issueInput = document.getElementById('issueInput');
const setIssueBtn = document.getElementById('setIssueBtn');
const clearIssueBtn = document.getElementById('clearIssueBtn');
const refreshBtn = document.getElementById('refreshBtn');
const quickActionBtns = document.querySelectorAll('[data-issue]');
const exportCsvBtn = document.getElementById('exportCsvBtn');
const exportJsonBtn = document.getElementById('exportJsonBtn');

// State
let currentStatus = null;

// Initialize app
document.addEventListener('DOMContentLoaded', () => {
    console.log('App initialized');
    loadStatus();
    setupEventListeners();
});

// Set up event listeners
function setupEventListeners() {
    setIssueBtn.addEventListener('click', handleSetIssue);
    clearIssueBtn.addEventListener('click', handleClearIssue);
    refreshBtn.addEventListener('click', handleRefresh);
    exportCsvBtn.addEventListener('click', () => handleExport('csv'));
    exportJsonBtn.addEventListener('click', () => handleExport('json'));

    // Quick action buttons
    quickActionBtns.forEach(btn => {
        btn.addEventListener('click', (e) => {
            const issue = e.target.getAttribute('data-issue');
            setIssue(issue);
        });
    });

    // Enter key in input
    issueInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            handleSetIssue();
        }
    });
}

// Load status from daemon
async function loadStatus() {
    try {
        showLoading(true);

        // Get daemon status
        const status = await invoke('get_status');
        currentStatus = status;

        // Get activity summary
        const activity = await invoke('get_activity_summary');

        // Update UI
        updateStatusUI(status, activity);
        updateDaemonStatus(true);

    } catch (error) {
        console.error('Failed to load status:', error);
        updateDaemonStatus(false);
        showError('Failed to connect to daemon. Make sure it\'s running.');
    } finally {
        showLoading(false);
    }
}

// Update status UI
function updateStatusUI(status, activity) {
    // Current issue
    if (activity.current_issue) {
        currentIssueEl.textContent = activity.current_issue;
        currentIssueEl.style.color = 'var(--primary)';
    } else {
        currentIssueEl.textContent = 'Auto-detect';
        currentIssueEl.style.color = 'var(--text-light)';
    }

    // Tracking status
    if (activity.is_tracking) {
        trackingStatusEl.textContent = 'Tracking';
        trackingStatusEl.className = 'status-badge tracking';
    } else {
        trackingStatusEl.textContent = 'Not Tracking';
        trackingStatusEl.className = 'status-badge not-tracking';
    }

    // Total time
    totalTimeEl.textContent = activity.total_tracked_today;
}

// Update daemon status indicator
function updateDaemonStatus(isOnline) {
    if (isOnline) {
        daemonStatusEl.textContent = 'Online';
        daemonStatusEl.className = 'daemon-status online';
    } else {
        daemonStatusEl.textContent = 'Offline';
        daemonStatusEl.className = 'daemon-status offline';
    }
}

// Handle set issue
async function handleSetIssue() {
    const issueKey = issueInput.value.trim();

    if (!issueKey) {
        showError('Please enter an issue key');
        return;
    }

    await setIssue(issueKey);
}

// Set issue override
async function setIssue(issueKey) {
    try {
        showLoading(true);

        await invoke('set_issue_override', { issueKey });

        // Clear input
        issueInput.value = '';

        // Reload status
        await loadStatus();

        showSuccess(`Issue set to ${issueKey}`);
    } catch (error) {
        console.error('Failed to set issue:', error);
        showError('Failed to set issue override');
    } finally {
        showLoading(false);
    }
}

// Handle clear issue
async function handleClearIssue() {
    try {
        showLoading(true);

        await invoke('set_issue_override', { issueKey: null });

        // Clear input
        issueInput.value = '';

        // Reload status
        await loadStatus();

        showSuccess('Issue override cleared');
    } catch (error) {
        console.error('Failed to clear issue:', error);
        showError('Failed to clear issue override');
    } finally {
        showLoading(false);
    }
}

// Handle refresh
async function handleRefresh() {
    refreshBtn.style.transform = 'rotate(180deg)';
    await loadStatus();
    setTimeout(() => {
        refreshBtn.style.transform = 'rotate(0deg)';
    }, 200);
}

// Show loading state
function showLoading(isLoading) {
    const buttons = document.querySelectorAll('.btn');
    buttons.forEach(btn => {
        btn.disabled = isLoading;
        btn.style.opacity = isLoading ? '0.6' : '1';
    });
    issueInput.disabled = isLoading;
}

// Show error message
function showError(message) {
    // Simple console error for now - can be enhanced with UI notifications
    console.error(message);
    // TODO: Add toast notification system
}

// Show success message
function showSuccess(message) {
    // Simple console log for now - can be enhanced with UI notifications
    console.log(message);
    // TODO: Add toast notification system
}

// Handle export
async function handleExport(format) {
    try {
        showLoading(true);

        const data = await invoke('export_data', { format });

        // Create a blob and download it
        const blob = new Blob([data], {
            type: format === 'csv' ? 'text/csv' : 'application/json'
        });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `activity-export-${new Date().toISOString().split('T')[0]}.${format}`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        showSuccess(`Data exported as ${format.toUpperCase()}`);
    } catch (error) {
        console.error('Failed to export data:', error);
        showError(`Failed to export data: ${error}`);
    } finally {
        showLoading(false);
    }
}

// Auto-refresh every 30 seconds
setInterval(() => {
    loadStatus();
}, 30000);
