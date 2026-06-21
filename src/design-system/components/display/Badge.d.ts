import * as React from 'react';

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** Semantic color. @default "neutral" */
  tone?: 'neutral' | 'brand' | 'success' | 'warning' | 'danger' | 'info';
  /** @default "md" */
  size?: 'sm' | 'md';
  /** Show a leading status dot. */
  dot?: boolean;
  children?: React.ReactNode;
}

/**
 * Pill badge for job status, counts and categories. Map print-shop states
 * onto tones: queued→info, on press→brand, shipped→success, overdue→danger,
 * awaiting art→warning.
 *
 * @startingPoint section="Display" subtitle="Status badges & tones" viewport="700x130"
 */
export function Badge(props: BadgeProps): JSX.Element;
