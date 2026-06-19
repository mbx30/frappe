import * as React from 'react';

export interface AvatarProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** Full name — drives initials and the deterministic fallback color. */
  name?: string;
  /** Image URL; falls back to initials when absent or while loading. */
  src?: string;
  /** @default "md" */
  size?: 'xs' | 'sm' | 'md' | 'lg';
}

export interface AvatarGroupProps {
  /** Names to render as overlapping avatars. */
  names?: string[];
  /** Max avatars before collapsing into +N. @default 4 */
  max?: number;
  size?: 'xs' | 'sm' | 'md' | 'lg';
}

/** Circular avatar with image or auto-colored initials. */
export function Avatar(props: AvatarProps): JSX.Element;
/** Overlapping avatar stack with +N overflow. */
export function AvatarGroup(props: AvatarGroupProps): JSX.Element;
