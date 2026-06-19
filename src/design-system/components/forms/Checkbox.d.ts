import * as React from 'react';

export interface CheckboxProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type'> {
  /** Text label to the right of the box. */
  label?: React.ReactNode;
  /** Secondary description below the label. */
  hint?: string;
  /** Render the mixed/dash state. */
  indeterminate?: boolean;
}

/**
 * Labelled checkbox with a violet checked state and an indeterminate dash.
 * Works controlled (`checked`) or uncontrolled (`defaultChecked`).
 */
export function Checkbox(props: CheckboxProps): JSX.Element;
