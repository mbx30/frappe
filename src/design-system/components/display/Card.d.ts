import * as React from 'react';

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Header title. */
  title?: React.ReactNode;
  /** Header subtitle below the title. */
  subtitle?: React.ReactNode;
  /** Header action node(s), right-aligned (e.g. an IconButton). */
  actions?: React.ReactNode;
  /** Footer content on a subtle inset bar. */
  footer?: React.ReactNode;
  /** Hover elevation + lift for clickable cards. */
  interactive?: boolean;
  /** Body padding. @default "md" */
  padding?: 'none' | 'sm' | 'md' | 'lg';
  /** Style for the body wrapper. */
  bodyStyle?: React.CSSProperties;
  children?: React.ReactNode;
}

/**
 * Surface container with optional header (title/subtitle/actions), body and
 * footer. The workhorse panel for dashboards, order summaries, settings.
 */
export function Card(props: CardProps): JSX.Element;
