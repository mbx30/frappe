import * as React from 'react';

export interface TagProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** Optional leading color swatch (e.g. a customer or label color). */
  color?: string;
  /** Optional leading icon node. */
  icon?: React.ReactNode;
  /** Show a dismiss button and call this when clicked. */
  onRemove?: (e: React.MouseEvent) => void;
  children?: React.ReactNode;
}

/**
 * Squared chip for filters, attributes and multi-select tokens. Quieter than
 * Badge. Pass `onRemove` to make it dismissible; `color` for a label swatch.
 */
export function Tag(props: TagProps): JSX.Element;
