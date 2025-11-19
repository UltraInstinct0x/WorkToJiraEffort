// Tauri API
const { invoke } = window.__TAURI__.core;

// DOM Elements
const currentIssueEl = document.getElementById('currentIssue');
const trackingStatusEl = document.getElementById('trackingStatus');
const trackingStatusTextEl = document.getElementById('trackingStatusText');
const totalTimeEl = document.getElementById('totalTime');
const connectionIndicatorEl = document.getElementById('connectionIndicator');
const connectionTextEl = document.getElementById('connectionText');
const issueInput = document.getElementById('issueInput');
const setIssueBtn = document.getElementById('setIssueBtn');
const clearIssueBtn = document.getElementById('clearIssueBtn');
const openDashboardBtn = document.getElementById('openDashboardBtn');

// Load status
async function loadStatus() {
    try {
        const status = await invoke('get_status');
        const activity = await invoke('get_activity_summary');
        
        // Update UI
        if (activity.current_issue) {
            currentIssueEl.textContent = activity.current_issue;
        } else {
            currentIssueEl.textContent = 'Auto-detect';
        }
        
        if (activity.is_tracking) {
            trackingStatusTextEl.textContent = 'Active';
            trackingStatusEl.classList.add('status-badge--active');
        } else {
            trackingStatusTextEl.textContent = 'Inactive';
            trackingStatusEl.classList.remove('status-badge--active');
        }
        
        totalTimeEl.textContent = activity.total_tracked_today;
        
        connectionIndicatorEl.classList.add('connection-status__indicator--online');
        connectionTextEl.textContent = 'Connected';
    } catch (error) {
        console.error('Failed to load status:', error);
        connectionIndicatorEl.classList.remove('connection-status__indicator--online');
        connectionIndicatorEl.classList.add('connection-status__indicator--offline');
        connectionTextEl.textContent = 'Offline';
    }
}

// Set issue
setIssueBtn.addEventListener('click', async () => {
    const issueKey = issueInput.value.trim();
    if (!issueKey) return;
    
    try {
        await invoke('set_issue_override', { issueKey });
        issueInput.value = '';
        await loadStatus();
    } catch (error) {
        console.error('Failed to set issue:', error);
    }
});

// Clear issue
clearIssueBtn.addEventListener('click', async () => {
    try {
        await invoke('set_issue_override', { issueKey: null });
        issueInput.value = '';
        await loadStatus();
    } catch (error) {
        console.error('Failed to clear issue:', error);
    }
});

// Open dashboard
openDashboardBtn.addEventListener('click', async () => {
    try {
        await invoke('open_dashboard');
    } catch (error) {
        console.error('Failed to open dashboard:', error);
    }
});

// Enter key
issueInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        setIssueBtn.click();
    }
});

// Init
document.addEventListener('DOMContentLoaded', () => {
    loadStatus();
    setInterval(loadStatus, 30000);
});
