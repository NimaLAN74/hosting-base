# Lianel World Design System

## Color Palette

Based on the `lw-background.png` image, the following color palette has been extracted and implemented:

### Primary Colors
- **Primary Blue**: `#2c5aa0` - Main brand color, used for primary elements
- **Secondary Blue**: `#4a7bc8` - Lighter blue for secondary elements
- **Dark Blue**: `#1b4f72` - Darker shade for depth and contrast

### Accent Colors
- **Accent Gold**: `#f4d03f` - Highlight color for important elements and CTAs
- **Light Blue**: `#85c1e9` - Subtle blue for text and secondary information

### Neutral Colors
- **White**: `#ffffff` - Primary text and card backgrounds
- **Light Gray**: `#f8f9fa` - Light backgrounds and subtle elements
- **Dark Gray**: `#2c3e50` - Dark text and secondary content

### Utility Colors
- **Shadow**: `rgba(0, 0, 0, 0.2)` - Standard shadow
- **Shadow Hover**: `rgba(0, 0, 0, 0.3)` - Enhanced shadow on hover

## Typography

### Font Weights
- **Light**: 300 - Subtitles and secondary text
- **Regular**: 400 - Body text
- **Semi-Bold**: 600 - Card headings
- **Bold**: 700 - Main headings and logo

### Font Sizes
- **Logo**: 2.5rem (40px)
- **Main Title**: 3.5rem (56px)
- **Subtitle**: 1.5rem (24px)
- **Card Heading**: 1.5rem (24px)
- **Body Text**: 1rem (16px)
- **Footer**: 0.9rem (14.4px)

## Components

### Logo
- **Icon**: "LW" in gold on blue background
- **Typography**: "Lianel World" with letter spacing
- **Colors**: White text with gold accent icon

### Cards
- **Background**: Semi-transparent white with blur effect
- **Border Radius**: 16px
- **Shadow**: Layered shadows for depth
- **Hover Effect**: Lift animation with enhanced shadow

### Service Cards
- **Background**: Blue gradient
- **Text**: White with gold accents
- **Border**: Transparent with gold hover border
- **Icons**: Gold colored for contrast

## Layout

### Grid System
- **Max Width**: 1200px
- **Responsive Breakpoints**:
  - Mobile: 480px and below
  - Tablet: 768px and below
  - Desktop: 769px and above

### Spacing
- **Section Gap**: 60px
- **Card Gap**: 30px
- **Internal Padding**: 40px (desktop), 30px (mobile)

## Effects

### Glassmorphism
- **Backdrop Filter**: blur(10px)
- **Background**: Semi-transparent white
- **Border**: Subtle white border

### Animations
- **Hover Lift**: translateY(-5px)
- **Transition Duration**: 0.3s ease
- **Logo Hover**: rotate(5deg) scale(1.1)

## Background

### Primary Background
- **Image**: `lw-background.png` - Fixed attachment, center cover
- **Overlay**: Blue gradient overlay (70-80% opacity) for text readability
- **Fallback**: Blue gradient if image fails to load

## Accessibility

### Contrast Ratios
- White text on blue background: AAA compliant
- Dark text on white cards: AAA compliant
- Gold accents provide sufficient contrast

### Interactive Elements
- Clear hover states
- Focus indicators
- Semantic HTML structure
- Alt text for images

## Usage Guidelines

### Do's
- Use the primary blue for main navigation and CTAs
- Apply gold sparingly for highlights and important elements
- Maintain consistent spacing and typography
- Use glassmorphism effects for modern feel

### Don'ts
- Don't use colors outside the defined palette
- Don't reduce contrast below accessibility standards
- Don't overuse the gold accent color
- Don't break the grid system without purpose

## Implementation

The design system is implemented using CSS custom properties (variables) for easy maintenance and consistency across the application.

```css
:root {
  --primary-blue: #2c5aa0;
  --secondary-blue: #4a7bc8;
  --accent-gold: #f4d03f;
  /* ... other variables */
}
```

This allows for easy theme updates and ensures consistency across all components.