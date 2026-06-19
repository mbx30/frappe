import * as React from 'react';

export interface IconButtonProps extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'aria-label'> {
  /** The glyph to render (e.g. a Lucide icon element). */
  icon: React.ReactNode;
  /** Accessible label — surfaced as aria-label and native tooltip. */
  label: string;
  /** @default "ghost" */
  variant?: 'primary' | 'secondary' | 'subtle' | 'ghost';
  /** @default "md" */
  size?: 'sm' | 'md' | 'lg';
}

/**
 * Square, icon-only button for toolbars and row actions. Always pass a
 * `label` — it becomes the aria-label and hover tooltip.
 */
export function IconButton(props: IconButtonProps): JSX.Element;
