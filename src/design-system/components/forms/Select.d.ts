import * as React from 'react';

export type SelectOption = string | { value: string; label: string };

export interface SelectProps extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, 'size'> {
  label?: string;
  hint?: string;
  error?: string;
  /** @default "md" */
  size?: 'sm' | 'md' | 'lg';
  /** Options as strings or `{ value, label }`. You may pass `<option>` children instead. */
  options?: SelectOption[];
  /** Disabled first option shown when nothing is selected. */
  placeholder?: string;
  containerStyle?: React.CSSProperties;
}

/**
 * Native `<select>` restyled to match Frappe inputs, with a chevron and
 * the same focus/error treatment as `Input`.
 */
export function Select(props: SelectProps): JSX.Element;
