# UI Components Documentation

## Overview
Component library for WorkToJiraEffort UI, inspired by TimeScribe's design patterns.

---

## Status Dashboard

### Purpose
Display current tracking status, active issue, and connection health.

### Structure
```html
<div class="status-dashboard">
  <div class="status-header">
    <h2>Current Status</h2>
    <button class="btn-refresh" aria-label="Refresh status">↻</button>
  </div>

  <div class="status-grid">
    <div class="status-item">
      <span class="status-label">Tracking</span>
      <span class="status-badge status-badge--active">
        <svg class="icon">...</svg>
        Active
      </span>
    </div>

    <div class="status-item">
      <span class="status-label">Current Issue</span>
      <span class="status-value">PROJ-123</span>
    </div>

    <div class="status-item">
      <span class="status-label">Time Today</span>
      <span class="status-value">2h 34m</span>
    </div>
  </div>
</div>
```

### States
- **Active**: Green sage badge, pulse animation
- **Idle**: Gray badge, no animation
- **Error**: Red badge, warning icon

### Styling
```css
.status-dashboard {
  background: var(--color-surface);
  border-radius: var(--radius-md);
  padding: var(--space-4);
  box-shadow: var(--shadow-md);
}
```

---

## Recent Issues List

### Purpose
Quick access to recently tracked issues.

### Structure
```html
<div class="recent-issues">
  <h3 class="recent-issues__title">Recent Issues</h3>

  <ul class="recent-issues__list">
    <li class="issue-card">
      <div class="issue-card__header">
        <span class="issue-card__key">PROJ-123</span>
        <span class="issue-card__time">2h 15m</span>
      </div>
      <p class="issue-card__title">Implement user authentication</p>
      <button class="issue-card__action">Track This</button>
    </li>
  </ul>
</div>
```

### Features
- Last 5-10 issues
- Shows total time per issue
- Click to set as active
- Smooth slide-in animation

### Data Source
```javascript
// Stored in localStorage
const recentIssues = [
  {
    key: 'PROJ-123',
    title: 'Issue title from Jira',
    totalTime: 135, // minutes
    lastUsed: Date.now()
  }
];
```

---

## Time Summary Visualization

### Purpose
Visual breakdown of today's tracked time by issue.

### Structure
```html
<div class="time-summary">
  <h3 class="time-summary__title">Today's Summary</h3>

  <div class="time-summary__total">
    <span class="time-summary__label">Total Tracked</span>
    <span class="time-summary__value">5h 42m</span>
  </div>

  <div class="time-summary__chart">
    <div class="time-bar" style="--percentage: 45%; --color: var(--color-terracotta)">
      <span class="time-bar__label">PROJ-123</span>
      <span class="time-bar__time">2h 34m</span>
    </div>
    <div class="time-bar" style="--percentage: 35%; --color: var(--color-sage)">
      <span class="time-bar__label">PROJ-456</span>
      <span class="time-bar__time">2h 00m</span>
    </div>
    <div class="time-bar" style="--percentage: 20%; --color: var(--color-info)">
      <span class="time-bar__label">PROJ-789</span>
      <span class="time-bar__time">1h 08m</span>
    </div>
  </div>
</div>
```

### Styling
```css
.time-bar {
  width: var(--percentage);
  height: 48px;
  background: var(--color);
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--space-3);
  transition: transform var(--duration-normal);
}

.time-bar:hover {
  transform: scaleX(1.02);
}
```

---

## Notification Preferences

### Purpose
Control notification behavior for time tracking.

### Structure
```html
<div class="notification-prefs">
  <h3 class="notification-prefs__title">Notifications</h3>

  <div class="pref-toggle">
    <label class="pref-toggle__label">
      <input type="checkbox" class="pref-toggle__input" checked>
      <span class="pref-toggle__slider"></span>
      <span class="pref-toggle__text">Enable Notifications</span>
    </label>
  </div>

  <div class="pref-option">
    <label>
      <input type="radio" name="frequency" value="immediate">
      Immediate (every activity change)
    </label>
  </div>

  <div class="pref-option">
    <label>
      <input type="radio" name="frequency" value="hourly" checked>
      Hourly Summary
    </label>
  </div>

  <div class="pref-option">
    <label>
      <input type="radio" name="frequency" value="daily">
      Daily Summary Only
    </label>
  </div>
</div>
```

### Toggle Switch Styling
```css
.pref-toggle__input {
  position: absolute;
  opacity: 0;
}

.pref-toggle__slider {
  width: 44px;
  height: 24px;
  background: var(--color-border);
  border-radius: var(--radius-full);
  position: relative;
  transition: background var(--duration-normal);
}

.pref-toggle__slider::after {
  content: '';
  width: 20px;
  height: 20px;
  background: white;
  border-radius: 50%;
  position: absolute;
  left: 2px;
  top: 2px;
  transition: transform var(--duration-normal);
}

.pref-toggle__input:checked + .pref-toggle__slider {
  background: var(--color-sage);
}

.pref-toggle__input:checked + .pref-toggle__slider::after {
  transform: translateX(20px);
}
```

---

## Issue Selector

### Purpose
Set or change the active tracking issue.

### Structure
```html
<div class="issue-selector">
  <label class="issue-selector__label">Active Issue</label>

  <div class="issue-selector__input-group">
    <input
      type="text"
      class="issue-selector__input"
      placeholder="e.g., PROJ-123"
      maxlength="50"
    >
    <button class="btn btn--primary">Set</button>
  </div>

  <button class="btn btn--ghost btn--sm issue-selector__clear">
    Clear Override
  </button>
</div>
```

### Validation
```javascript
function validateIssueKey(key) {
  const pattern = /^[A-Z]+-\d+$/;
  return pattern.test(key);
}
```

---

## Connection Status Footer

### Purpose
Show daemon connectivity and sync status.

### Structure
```html
<footer class="app-footer">
  <div class="connection-status">
    <div class="connection-status__indicator connection-status__indicator--online"></div>
    <span class="connection-status__text">Daemon Online</span>
  </div>

  <div class="sync-status">
    <span class="sync-status__text">Last sync: 2m ago</span>
  </div>
</footer>
```

### Indicator States
```css
.connection-status__indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}

.connection-status__indicator--online {
  background: var(--color-success);
  animation: pulse 2s ease-in-out infinite;
}

.connection-status__indicator--offline {
  background: var(--color-error);
}

.connection-status__indicator--connecting {
  background: var(--color-warning);
  animation: pulse 1s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
```

---

## Empty States

### Purpose
Friendly messages when no data is available.

### Structure
```html
<div class="empty-state">
  <svg class="empty-state__icon">...</svg>
  <h3 class="empty-state__title">No Issues Yet</h3>
  <p class="empty-state__description">
    Start tracking to see your recent issues here.
  </p>
</div>
```

---

## Loading States

### Purpose
Indicate async operations in progress.

### Skeleton Loader
```html
<div class="skeleton">
  <div class="skeleton__line"></div>
  <div class="skeleton__line skeleton__line--short"></div>
</div>
```

```css
.skeleton__line {
  height: 16px;
  background: linear-gradient(
    90deg,
    var(--color-border) 25%,
    var(--color-surface-secondary) 50%,
    var(--color-border) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
  border-radius: var(--radius-sm);
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
```

---

## Buttons

### Variants
```html
<!-- Primary -->
<button class="btn btn--primary">Primary Action</button>

<!-- Secondary -->
<button class="btn btn--secondary">Secondary Action</button>

<!-- Ghost -->
<button class="btn btn--ghost">Ghost Action</button>

<!-- Danger -->
<button class="btn btn--danger">Delete</button>

<!-- With Icon -->
<button class="btn btn--primary">
  <svg class="icon">...</svg>
  With Icon
</button>
```

### Sizes
```html
<button class="btn btn--sm">Small</button>
<button class="btn">Default</button>
<button class="btn btn--lg">Large</button>
```

---

## Best Practices

1. **Consistency**: Use components as defined, avoid one-offs
2. **Accessibility**: Include ARIA labels, keyboard support
3. **States**: Always show loading, error, empty states
4. **Animations**: Smooth transitions, respect reduced-motion
5. **Responsive**: Components adapt to container width
6. **Semantic HTML**: Use correct elements for meaning
7. **Icons**: Consistent size and stroke width
8. **Colors**: Use design system tokens, not hardcoded values
