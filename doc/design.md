# Codictate Design Documentation

This document outlines the design principles and component rules for the Codictate application, ensuring a consistent, premium, and system-native user experience.

## Core Aesthetic
**"Clean, Minimal, System-Native"**
The interface aims to feel like a premium, native system component. It prioritizes clarity, legibility, and a clutter-free visual hierarchy.

## Design Rules & Components

### 1. Settings Groups
- **Layout**: Follows the "System Settings" paradigm (macOS/iOS style).
- **Structure**:
    - **Container**: A seamless, solid card (`bg-card`, `border`, `shadow-sm`, `rounded-xl`).
    - **Header**: Group titles and descriptions must reside **outside** the card border. This avoids internal "header bars" or dividers that create visual noise.
    - **Content**: The card itself contains only the list of settings rows.

### 2. Buttons
- **Shape**: **`rounded-md`** ("Square-ish but friendly"). Avoid full pills or sharp rectangles.
- **Dimensions**:
    - **Width**: Minimum `12rem` (`min-w-[12rem]`) for a wide, substantial target.
    - **Padding**: Generous horizontal padding (`px-6`).
- **Texture & Depth**:
    - **Shadow**: `shadow-sm` provides a subtle tactile lift, crucial for depth in dark mode.
- **Color & Semantics**:
    - **Standard Action**: **`bg-secondary`** (Solid Gray). Provides high visibility and contrast in dark mode.
    - **Destructive Action**: **`bg-destructive`** (Red). Use for irreversible or dangerous actions (e.g., Reset).

### 3. Typography
- **Weight**: **`font-medium`**. Strikes the balance between legibility and visual weight (avoiding the heaviness of Bold and the thinness of Regular).
- **Sizes**:
    - **Titles**: `text-sm` in `text-foreground/90` for clear contrast.
    - **Descriptions**: `text-[13px]` with `leading-relaxed` for easy scanning without dominating the UI.

### 4. Layout & Spacing
- **Breathing Room**: Increased gap between text labels and action controls (`mr-6`).
- **Alignment**: Ensure vertical center alignment for all row elements, including tooltips and icons.
