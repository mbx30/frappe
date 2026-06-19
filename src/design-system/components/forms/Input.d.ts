import * as React from 'react';

export interface InputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size'> {
  /** Field label shown above the control. */
  label?: string;
  /** Helper text shown below when there is no error. */
  hint?: string;
  /** Error message — turns the border red and replaces the hint. */
  error?: string;
  /** @default "md" */
  size?: 'sm' | 'md' | 'lg';
  /** Leading icon node. */
  iconLeft?: React.ReactNode;
  /** Trailing unit/suffix text (e.g. "qty", "in"). */
  suffix?: React.ReactNode;
  /** Render the value in Geist Mono with tabular figures — for $, IDs, qty. */
  mono?: boolean;
  /** Style applied to the outer wrapper. */
  containerStyle?: React.CSSProperties;
}

/**
 * Single-line text field with label, hint/error, leading icon and suffix.
 * Set `mono` for numeric/price/ID inputs.
 */
export function Input(props: InputProps): JSX.Element;
