import * as React from 'react';

export interface SwitchProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type' | 'size'> {
  /** Optional label to the right of the track. */
  label?: React.ReactNode;
  /** @default "md" */
  size?: 'sm' | 'md';
}

/**
 * Toggle switch for instant-apply settings (auto-print labels, dark mode).
 * Controlled with `checked`/`onChange` or uncontrolled with `defaultChecked`.
 */
export function Switch(props: SwitchProps): JSX.Element;
