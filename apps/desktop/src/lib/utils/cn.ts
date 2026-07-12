/**
 * Tailwind class-name merge helper. Combines `clsx`-style conditional logic
 * with `tailwind-merge` to dedupe conflicting utilities.
 */
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}