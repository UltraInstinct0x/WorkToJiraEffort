// Tauri API
const { invoke } = window.__TAURI__.core;

// Constants
const MAX_RECENT_ISSUES = 10;
const AUTO_REFRESH_INTERVAL = 30000; // 30 seconds
const STORAGE_KEYS = {
    RECENT_ISSUES: 'worktojira_recent_issues',
    NOTIFICATION_PREFS: 'worktojira_notification_prefs',
    THEME: 'worktojira_theme'
};

// DOM Elements
const currentIssueEl = document.getElementById('currentIssue');
const trackingStatusEl = document.getElementById('trackingStatus');
const trackingStatusTextEl = document.getElementById('trackingStatusText');
const totalTimeEl = document.getElementById('totalTime');
const issueInput = document.getElementById('issueInput');
const setIssueBtn = document.getElementById('setIssueBtn');
const clearIssueBtn = document.getElementById('clearIssueBtn');
const refreshBtn = document.getElementById('refreshBtn');
const themeToggle = document.getElementById('themeToggle');
const appContainer = document.querySelector('.app');

// Recent Issues
const recentIssuesList = document.getElementById('recentIssuesList');
const recentIssuesCount = document.getElementById('recentIssuesCount');

// Time Summary
const timeSummaryChart = document.getElementById('timeSummaryChart');
const summaryTotalTime = document.getElementById('summaryTotalTime');

// Connection Status
const connectionIndicator = document.getElementById('connectionIndicator');
const connectionText = document.getElementById('connectionText');
const syncText = document.getElementById('syncText');

// Notification Preferences
const notificationsEnabled = document.getElementById('notificationsEnabled');
const notificationFrequency = document.getElementById('notificationFrequency');

// State
let currentStatus = null;
let autoRefreshTimer = null;
let lastSyncTime = null;
let isOnline = false;

// Initialize app
document.addEventListener('DOMContentLoaded', () => {
    console.log('WorkToJiraEffort UI initialized');
    initializeApp();
});

/**
 * Initialize the application
 */
async function initializeApp() {
    // Initialize theme
    initializeTheme();

    // Load notification preferences
    loadNotificationPreferences();

    // Load recent issues
    loadRecentIssues();

    // Setup event listeners
    setupEventListeners();

    // Initial status load
    await loadStatus();

    // Start auto-refresh
    startAutoRefresh();
}

/**
 * Setup all event listeners
 */
function setupEventListeners() {
    // Issue management
    setIssueBtn.addEventListener('click', handleSetIssue);
    clearIssueBtn.addEventListener('click', handleClearIssue);
    refreshBtn.addEventListener('click', handleRefresh);

    // Enter key in input
    issueInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            handleSetIssue();
        }
    });

    // Theme toggle
    themeToggle.addEventListener('click', toggleTheme);

    // Notification preferences
    notificationsEnabled.addEventListener('change', handleNotificationToggle);

    const frequencyRadios = document.querySelectorAll('input[name="frequency"]');
    frequencyRadios.forEach(radio => {
        radio.addEventListener('change', handleNotificationFrequencyChange);
    });
}

// ============================================================================
// Theme Management
// ============================================================================

/**
 * Initialize theme based on user preference or system settings
 */
function initializeTheme() {
    const savedTheme = localStorage.getItem(STORAGE_KEYS.THEME);

    let theme;
    if (savedTheme) {
        theme = savedTheme;
    } else {
        // Detect system preference
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        theme = prefersDark ? 'dark' : 'light';
    }

    setTheme(theme);

    // Listen for system theme changes
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
        if (!localStorage.getItem(STORAGE_KEYS.THEME)) {
            setTheme(e.matches ? 'dark' : 'light');
        }
    });
}

/**
 * Toggle theme between light and dark
 */
function toggleTheme() {
    const currentTheme = appContainer.getAttribute('data-theme');
    const newTheme = currentTheme === 'light' ? 'dark' : 'light';
    setTheme(newTheme);
    localStorage.setItem(STORAGE_KEYS.THEME, newTheme);
}

/**
 * Set the application theme
 */
function setTheme(theme) {
    appContainer.setAttribute('data-theme', theme);

    // Smooth transition
    appContainer.style.transition = 'background-color 0.3s ease, color 0.3s ease';
}

// ============================================================================
// Status Loading and Updates
// ============================================================================

/**
 * Load status from daemon
 */
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
        updateConnectionStatus(true);
        updateSyncTime();

        // Update time summary
        updateTimeSummary(activity);

        // Track current issue if exists
        if (activity.current_issue) {
            trackRecentIssue(activity.current_issue, activity.total_tracked_today);
        }

    } catch (error) {
        console.error('Failed to load status:', error);
        updateConnectionStatus(false);
        showError('Failed to connect to daemon. Make sure it is running.');
    } finally {
        showLoading(false);
    }
}

/**
 * Update status UI elements
 */
function updateStatusUI(status, activity) {
    // Current issue
    if (activity.current_issue) {
        currentIssueEl.textContent = activity.current_issue;
        currentIssueEl.classList.add('status-value--active');
    } else {
        currentIssueEl.textContent = 'Auto-detect';
        currentIssueEl.classList.remove('status-value--active');
    }

    // Tracking status
    if (activity.is_tracking) {
        trackingStatusTextEl.textContent = 'Active';
        trackingStatusEl.classList.add('status-badge--active');
        trackingStatusEl.classList.remove('status-badge--inactive');
    } else {
        trackingStatusTextEl.textContent = 'Not Active';
        trackingStatusEl.classList.remove('status-badge--active');
        trackingStatusEl.classList.add('status-badge--inactive');
    }

    // Total time
    totalTimeEl.textContent = formatTime(activity.total_tracked_today);
}

/**
 * Update connection status indicator
 */
function updateConnectionStatus(online) {
    isOnline = online;

    if (online) {
        connectionIndicator.classList.add('connection-status__indicator--online');
        connectionIndicator.classList.remove('connection-status__indicator--offline');
        connectionText.textContent = 'Connected';
    } else {
        connectionIndicator.classList.remove('connection-status__indicator--online');
        connectionIndicator.classList.add('connection-status__indicator--offline');
        connectionText.textContent = 'Disconnected';
    }
}

/**
 * Update last sync timestamp
 */
function updateSyncTime() {
    lastSyncTime = new Date();
    syncText.textContent = `Last sync: ${formatSyncTime(lastSyncTime)}`;
}

/**
 * Format sync time for display
 */
function formatSyncTime(date) {
    const now = new Date();
    const diffMs = now - date;
    const diffSecs = Math.floor(diffMs / 1000);
    const diffMins = Math.floor(diffSecs / 60);

    if (diffSecs < 60) {
        return 'Just now';
    } else if (diffMins < 60) {
        return `${diffMins}m ago`;
    } else {
        return date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
    }
}

// ============================================================================
// Issue Management
// ============================================================================

/**
 * Handle set issue button click
 */
async function handleSetIssue() {
    const issueKey = issueInput.value.trim();

    if (!issueKey) {
        showError('Please enter an issue key');
        return;
    }

    // Validate issue key format (basic validation)
    if (!isValidIssueKey(issueKey)) {
        showError('Invalid issue key format. Use format like PROJ-123');
        return;
    }

    await setIssue(issueKey);
}

/**
 * Set issue override
 */
async function setIssue(issueKey) {
    try {
        showLoading(true);

        await invoke('set_issue_override', { issueKey });

        // Clear input
        issueInput.value = '';

        // Reload status
        await loadStatus();

        showSuccess(`Issue set to ${issueKey}`);

        // Track in recent issues
        trackRecentIssue(issueKey, '0m');

    } catch (error) {
        console.error('Failed to set issue:', error);
        showError(`Failed to set issue override: ${error.message || error}`);
    } finally {
        showLoading(false);
    }
}

/**
 * Handle clear issue button click
 */
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
        showError(`Failed to clear issue override: ${error.message || error}`);
    } finally {
        showLoading(false);
    }
}

/**
 * Validate issue key format
 */
function isValidIssueKey(key) {
    // Basic validation: PROJECT-123 format
    const pattern = /^[A-Z][A-Z0-9]*-\d+$/i;
    return pattern.test(key);
}

/**
 * Handle refresh button click
 */
async function handleRefresh() {
    // Animate refresh button
    refreshBtn.style.transform = 'rotate(360deg)';
    refreshBtn.style.transition = 'transform 0.5s ease';

    await loadStatus();

    setTimeout(() => {
        refreshBtn.style.transform = 'rotate(0deg)';
    }, 500);
}

// ============================================================================
// Recent Issues Management
// ============================================================================

/**
 * Track a recently used issue
 */
function trackRecentIssue(issueKey, totalTime) {
    let recentIssues = getRecentIssues();

    // Find existing issue or create new
    const existingIndex = recentIssues.findIndex(issue => issue.key === issueKey);

    if (existingIndex !== -1) {
        // Update existing
        recentIssues[existingIndex].totalTime = totalTime;
        recentIssues[existingIndex].lastUsed = new Date().toISOString();
    } else {
        // Add new issue
        recentIssues.unshift({
            key: issueKey,
            title: issueKey, // Could be enhanced with actual title from JIRA API
            totalTime: totalTime,
            lastUsed: new Date().toISOString()
        });
    }

    // Keep only MAX_RECENT_ISSUES
    recentIssues = recentIssues.slice(0, MAX_RECENT_ISSUES);

    // Save to localStorage
    saveRecentIssues(recentIssues);

    // Update UI
    renderRecentIssues(recentIssues);
}

/**
 * Get recent issues from localStorage
 */
function getRecentIssues() {
    try {
        const stored = localStorage.getItem(STORAGE_KEYS.RECENT_ISSUES);
        return stored ? JSON.parse(stored) : [];
    } catch (error) {
        console.error('Failed to load recent issues:', error);
        return [];
    }
}

/**
 * Save recent issues to localStorage
 */
function saveRecentIssues(issues) {
    try {
        localStorage.setItem(STORAGE_KEYS.RECENT_ISSUES, JSON.stringify(issues));
    } catch (error) {
        console.error('Failed to save recent issues:', error);
    }
}

/**
 * Load and render recent issues
 */
function loadRecentIssues() {
    const recentIssues = getRecentIssues();
    renderRecentIssues(recentIssues);
}

/**
 * Render recent issues list
 */
function renderRecentIssues(issues) {
    // Update count badge
    recentIssuesCount.textContent = issues.length;

    if (issues.length === 0) {
        recentIssuesList.innerHTML = `
            <div class="empty-state">
                <svg class="empty-state__icon" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <path d="M12 2L2 7l10 5 10-5-10-5z"></path>
                    <path d="M2 17l10 5 10-5"></path>
                    <path d="M2 12l10 5 10-5"></path>
                </svg>
                <p class="empty-state__text">No recent issues yet</p>
            </div>
        `;
        return;
    }

    const issueItems = issues.map(issue => {
        const lastUsedDate = new Date(issue.lastUsed);
        const timeAgo = getTimeAgo(lastUsedDate);

        return `
            <div class="recent-issue-item" data-issue="${issue.key}">
                <div class="recent-issue-item__content">
                    <span class="recent-issue-item__key">${issue.key}</span>
                    <span class="recent-issue-item__time">${timeAgo}</span>
                </div>
                <div class="recent-issue-item__meta">
                    <span class="recent-issue-item__duration">${formatTime(issue.totalTime)}</span>
                </div>
            </div>
        `;
    }).join('');

    recentIssuesList.innerHTML = issueItems;

    // Add click handlers
    document.querySelectorAll('.recent-issue-item').forEach(item => {
        item.addEventListener('click', (e) => {
            const issueKey = e.currentTarget.getAttribute('data-issue');
            issueInput.value = issueKey;
            setIssue(issueKey);
        });
    });
}

/**
 * Get time ago string
 */
function getTimeAgo(date) {
    const now = new Date();
    const diffMs = now - date;
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;

    return date.toLocaleDateString();
}

// ============================================================================
// Time Summary
// ============================================================================

/**
 * Update time summary visualization
 */
function updateTimeSummary(activity) {
    // Update total time
    summaryTotalTime.textContent = formatTime(activity.total_tracked_today);

    // For now, create a simple visualization
    // This could be enhanced with actual daily breakdown from the backend
    const dailySummary = createDailySummary(activity);
    renderTimeSummaryChart(dailySummary);
}

/**
 * Create daily summary data structure
 */
function createDailySummary(activity) {
    // This is a simplified version - in production, this would come from the backend
    const summary = [];

    if (activity.current_issue) {
        summary.push({
            issueKey: activity.current_issue,
            duration: activity.total_tracked_today,
            percentage: 100
        });
    }

    return summary;
}

/**
 * Render time summary chart
 */
function renderTimeSummaryChart(summary) {
    if (summary.length === 0) {
        timeSummaryChart.innerHTML = `
            <div class="empty-state empty-state--compact">
                <p class="empty-state__text">Start tracking to see your summary</p>
            </div>
        `;
        return;
    }

    const chartItems = summary.map(item => `
        <div class="time-summary-item">
            <div class="time-summary-item__header">
                <span class="time-summary-item__issue">${item.issueKey}</span>
                <span class="time-summary-item__duration">${formatTime(item.duration)}</span>
            </div>
            <div class="time-summary-item__bar">
                <div class="time-summary-item__fill" style="width: ${item.percentage}%"></div>
            </div>
        </div>
    `).join('');

    timeSummaryChart.innerHTML = chartItems;
}

// ============================================================================
// Notification Preferences
// ============================================================================

/**
 * Load notification preferences from localStorage
 */
function loadNotificationPreferences() {
    try {
        const stored = localStorage.getItem(STORAGE_KEYS.NOTIFICATION_PREFS);
        const prefs = stored ? JSON.parse(stored) : {
            enabled: true,
            frequency: 'hourly'
        };

        // Apply to UI
        notificationsEnabled.checked = prefs.enabled;

        const frequencyRadio = document.querySelector(`input[name="frequency"][value="${prefs.frequency}"]`);
        if (frequencyRadio) {
            frequencyRadio.checked = true;
        }

        // Update frequency options visibility
        updateNotificationFrequencyVisibility(prefs.enabled);

    } catch (error) {
        console.error('Failed to load notification preferences:', error);
    }
}

/**
 * Save notification preferences to localStorage
 */
function saveNotificationPreferences(prefs) {
    try {
        localStorage.setItem(STORAGE_KEYS.NOTIFICATION_PREFS, JSON.stringify(prefs));
    } catch (error) {
        console.error('Failed to save notification preferences:', error);
    }
}

/**
 * Handle notification toggle
 */
function handleNotificationToggle(e) {
    const enabled = e.target.checked;

    // Update frequency options visibility
    updateNotificationFrequencyVisibility(enabled);

    // Get current frequency
    const frequencyRadio = document.querySelector('input[name="frequency"]:checked');
    const frequency = frequencyRadio ? frequencyRadio.value : 'hourly';

    // Save preferences
    saveNotificationPreferences({ enabled, frequency });

    showSuccess(`Notifications ${enabled ? 'enabled' : 'disabled'}`);
}

/**
 * Handle notification frequency change
 */
function handleNotificationFrequencyChange(e) {
    const frequency = e.target.value;

    // Save preferences
    const enabled = notificationsEnabled.checked;
    saveNotificationPreferences({ enabled, frequency });

    showSuccess(`Notification frequency set to ${frequency}`);
}

/**
 * Update notification frequency options visibility
 */
function updateNotificationFrequencyVisibility(enabled) {
    notificationFrequency.style.opacity = enabled ? '1' : '0.5';
    notificationFrequency.style.pointerEvents = enabled ? 'auto' : 'none';
}

// ============================================================================
// Auto-refresh
// ============================================================================

/**
 * Start auto-refresh timer
 */
function startAutoRefresh() {
    stopAutoRefresh();

    autoRefreshTimer = setInterval(async () => {
        if (isOnline) {
            await loadStatus();

            // Update sync time display every second
            if (lastSyncTime) {
                syncText.textContent = `Last sync: ${formatSyncTime(lastSyncTime)}`;
            }
        }
    }, AUTO_REFRESH_INTERVAL);
}

/**
 * Stop auto-refresh timer
 */
function stopAutoRefresh() {
    if (autoRefreshTimer) {
        clearInterval(autoRefreshTimer);
        autoRefreshTimer = null;
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Format time duration
 */
function formatTime(timeStr) {
    if (!timeStr) return '0h 0m';

    // If already formatted, return as is
    if (typeof timeStr === 'string' && timeStr.includes('h')) {
        return timeStr;
    }

    // Parse minutes if it's a number
    if (typeof timeStr === 'number') {
        const hours = Math.floor(timeStr / 60);
        const mins = timeStr % 60;
        return `${hours}h ${mins}m`;
    }

    return timeStr;
}

/**
 * Show loading state
 */
function showLoading(isLoading) {
    const buttons = document.querySelectorAll('.btn');
    buttons.forEach(btn => {
        btn.disabled = isLoading;
    });
    issueInput.disabled = isLoading;

    if (isLoading) {
        refreshBtn.classList.add('btn--loading');
    } else {
        refreshBtn.classList.remove('btn--loading');
    }
}

/**
 * Show error message
 */
function showError(message) {
    console.error('Error:', message);

    // Create toast notification
    showToast(message, 'error');
}

/**
 * Show success message
 */
function showSuccess(message) {
    console.log('Success:', message);

    // Create toast notification
    showToast(message, 'success');
}

/**
 * Show toast notification
 */
function showToast(message, type = 'info') {
    // Remove existing toasts
    const existingToast = document.querySelector('.toast');
    if (existingToast) {
        existingToast.remove();
    }

    // Create toast element
    const toast = document.createElement('div');
    toast.className = `toast toast--${type}`;
    toast.textContent = message;

    // Add to DOM
    document.body.appendChild(toast);

    // Trigger animation
    setTimeout(() => {
        toast.classList.add('toast--show');
    }, 10);

    // Auto-remove after 3 seconds
    setTimeout(() => {
        toast.classList.remove('toast--show');
        setTimeout(() => {
            toast.remove();
        }, 300);
    }, 3000);
}

// ============================================================================
// Cleanup
// ============================================================================

/**
 * Cleanup on page unload
 */
window.addEventListener('beforeunload', () => {
    stopAutoRefresh();
});

// Update sync time display every second
setInterval(() => {
    if (lastSyncTime) {
        syncText.textContent = `Last sync: ${formatSyncTime(lastSyncTime)}`;
    }
}, 1000);
