import * as React from 'react';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  /** Visual emphasis. @default "secondary" */
  variant?: 'primary' | 'secondary' | 'subtle' | 'ghost' | 'danger';
  /** Control height. @default "md" */
  size?: 'sm' | 'md' | 'lg';
  /** Icon node rendered before the label. */
  iconLeft?: React.ReactNode;
  /** Icon node rendered after the label. */
  iconRight?: React.ReactNode;
  /** Replaces left icon with a spinner and disables the button. */
  loading?: boolean;
  /** Stretch to fill the container width. */
  fullWidth?: boolean;
  children?: React.ReactNode;
}

/**
 * Primary action control for Frappe. Brand-violet `primary`, neutral
 * `secondary`, tinted `subtle`, quiet `ghost`, and destructive `danger`.
 */
export function Button(props: ButtonProps): JSX.Element;
