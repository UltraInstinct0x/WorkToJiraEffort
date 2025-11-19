# UI Design System

## Overview
WorkToJiraEffort features a TimeScribe-inspired UI with warm, professional aesthetics designed for autonomous AI-powered time tracking.

## Design Philosophy
- **Professional Warmth**: Approachable yet sophisticated
- **Clarity First**: Information hierarchy that guides the eye
- **Purposeful Motion**: Animations that enhance understanding
- **Autonomous Focus**: UI emphasizes the automated nature of tracking

---

## Color Palette

### Primary Colors
```css
--color-terracotta: #E07A5F;        /* Primary accent - warm, energetic */
--color-terracotta-light: #F2A490;  /* Hover states, highlights */
--color-terracotta-dark: #C96A55;   /* Active states, pressed */

--color-sage: #81B29A;              /* Secondary accent - calm, balanced */
--color-sage-light: #9BC4B0;        /* Success states, positive feedback */
--color-sage-dark: #6B9A7F;         /* Tracking indicator, active states */
```

### Neutral Colors (Light Mode)
```css
--color-bg: #FAFAF9;                /* Main background - warm white */
--color-surface: #FFFFFF;           /* Cards, elevated surfaces */
--color-surface-secondary: #F5F5F4; /* Secondary surfaces */

--color-text: #292524;              /* Primary text - stone 800 */
--color-text-secondary: #78716C;    /* Secondary text - stone 500 */
--color-text-tertiary: #A8A29E;     /* Tertiary text - stone 400 */

--color-border: #E7E5E4;            /* Borders - stone 200 */
--color-border-strong: #D6D3D1;     /* Strong borders - stone 300 */
```

### Neutral Colors (Dark Mode)
```css
--color-bg: #1C1917;                /* Main background - stone 900 */
--color-surface: #292524;           /* Cards - stone 800 */
--color-surface-secondary: #44403C; /* Secondary surfaces - stone 700 */

--color-text: #FAFAF9;              /* Primary text - stone 50 */
--color-text-secondary: #A8A29E;    /* Secondary text - stone 400 */
--color-text-tertiary: #78716C;     /* Tertiary text - stone 500 */

--color-border: #44403C;            /* Borders - stone 700 */
--color-border-strong: #57534E;     /* Strong borders - stone 600 */
```

### Semantic Colors
```css
--color-success: #81B29A;           /* Success states */
--color-warning: #F4A261;           /* Warning states */
--color-error: #E76F51;             /* Error states */
--color-info: #2A9D8F;              /* Info states */
```

### Opacity Scale
```css
--opacity-disabled: 0.5;
--opacity-hover: 0.8;
--opacity-subtle: 0.6;
```

---

## Typography

### Font Families
```css
--font-display: 'Fraunces', serif;          /* Headings, emphasis */
--font-body: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto',
             'Helvetica Neue', Arial, sans-serif;
--font-mono: 'SF Mono', 'Monaco', 'Cascadia Code', 'Courier New', monospace;
```

### Type Scale
```css
--text-xs: 0.75rem;      /* 12px - Tiny labels */
--text-sm: 0.875rem;     /* 14px - Secondary text */
--text-base: 1rem;       /* 16px - Body text */
--text-lg: 1.125rem;     /* 18px - Subheadings */
--text-xl: 1.25rem;      /* 20px - Section titles */
--text-2xl: 1.5rem;      /* 24px - Page titles */
--text-3xl: 1.875rem;    /* 30px - Hero text */
```

### Font Weights
```css
--font-normal: 400;
--font-medium: 500;
--font-semibold: 600;
--font-bold: 700;
```

### Line Heights
```css
--leading-tight: 1.25;
--leading-snug: 1.375;
--leading-normal: 1.5;
--leading-relaxed: 1.625;
--leading-loose: 2;
```

---

## Spacing Scale

```css
--space-1: 0.25rem;   /* 4px */
--space-2: 0.5rem;    /* 8px */
--space-3: 0.75rem;   /* 12px */
--space-4: 1rem;      /* 16px */
--space-5: 1.25rem;   /* 20px */
--space-6: 1.5rem;    /* 24px */
--space-8: 2rem;      /* 32px */
--space-10: 2.5rem;   /* 40px */
--space-12: 3rem;     /* 48px */
--space-16: 4rem;     /* 64px */
```

---

## Border Radius

```css
--radius-sm: 0.375rem;   /* 6px - Small elements */
--radius-md: 0.5rem;     /* 8px - Cards, buttons */
--radius-lg: 0.75rem;    /* 12px - Large cards */
--radius-xl: 1rem;       /* 16px - Hero elements */
--radius-full: 9999px;   /* Pills, badges */
```

---

## Shadows

```css
--shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.05);
--shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.1);
--shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.1);
--shadow-xl: 0 20px 25px -5px rgb(0 0 0 / 0.1);
--shadow-inner: inset 0 2px 4px 0 rgb(0 0 0 / 0.05);
```

---

## Animation

### Timing Functions
```css
--ease-in: cubic-bezier(0.4, 0, 1, 1);
--ease-out: cubic-bezier(0, 0, 0.2, 1);
--ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
--ease-spring: cubic-bezier(0.68, -0.55, 0.265, 1.55);
```

### Durations
```css
--duration-fast: 150ms;
--duration-normal: 250ms;
--duration-slow: 350ms;
--duration-slower: 500ms;
```

### Keyframe Animations
- **Fade In**: Opacity 0 → 1
- **Slide In**: Transform translateY(10px) → 0
- **Scale In**: Transform scale(0.95) → 1
- **Pulse**: Subtle scale animation for active elements
- **Shimmer**: Loading state animation

---

## Component Patterns

### Status Badges
- Pill-shaped (full border radius)
- Icon + text combination
- Color-coded by state
- Subtle shadow for depth

### Cards
- White/elevated background
- 8px border radius
- Subtle shadow (md)
- 16px padding
- Hover: Increase shadow (lg)

### Buttons
- Primary: Terracotta background
- Secondary: Sage background
- Ghost: Transparent with border
- Text: No background or border
- All: 6px border radius, medium font weight

### Input Fields
- Border: 2px solid border color
- Border radius: 6px
- Padding: 10px 12px
- Focus: Terracotta border
- Error: Red border

### Timeline Blocks
- Height: 56px (14rem equivalent)
- Border radius: Full (pill shape)
- Background: Color-coded by type
- Hover: Scale 1.1, ring shadow
- Smooth transitions

---

## Accessibility

### Color Contrast
- All text meets WCAG AA (4.5:1 for normal, 3:1 for large)
- Interactive elements have clear focus states
- Color is never the only indicator

### Focus States
```css
--focus-ring: 0 0 0 3px var(--color-terracotta);
--focus-ring-offset: 2px;
```

### Motion
- Respects `prefers-reduced-motion`
- Essential animations only
- No auto-playing motion

---

## Dark Mode Strategy

### Detection
```javascript
// System preference
const darkMode = window.matchMedia('(prefers-color-scheme: dark)').matches;

// Manual toggle (localStorage override)
localStorage.setItem('theme', 'dark' | 'light');
```

### Implementation
- CSS custom properties swap values
- Smooth transition between modes (300ms)
- Persist user preference

---

## Grid & Layout

### Container
```css
max-width: 400px;  /* Match Tauri window width */
padding: 0 16px;
margin: 0 auto;
```

### Flexbox Patterns
- Stack: `flex-direction: column; gap: var(--space-4)`
- Row: `flex-direction: row; gap: var(--space-3)`
- Center: `justify-content: center; align-items: center`

---

## Icons

### Source
- Lucide icons (SVG)
- Consistent 16px or 24px size
- Stroke width: 2px
- Color: Inherit from parent

### Usage
```html
<svg class="icon" width="16" height="16" viewBox="0 0 24 24">
  <path d="..." stroke="currentColor" />
</svg>
```

---

## Best Practices

1. **Use CSS Custom Properties**: Enable theming and consistency
2. **Mobile-First**: Design works on small screens first
3. **Progressive Enhancement**: Core functionality without JS
4. **Semantic HTML**: Proper elements for accessibility
5. **Smooth Transitions**: All interactive elements transition smoothly
6. **Loading States**: Show feedback for async operations
7. **Empty States**: Friendly messages when no data
8. **Error Handling**: Clear, actionable error messages

---

## Design Inspiration

This design system draws inspiration from:
- **TimeScribe**: Clean time tracking UI patterns
- **Linear**: Professional dashboard aesthetics
- **Craft**: Warm, editorial design language
- **Stripe**: Clear information hierarchy
