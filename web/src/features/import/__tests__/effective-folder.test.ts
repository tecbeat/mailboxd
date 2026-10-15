import { describe, expect, it } from 'vitest';
import { resolveEffectiveFolder } from '../effective-folder';

const base = { isPst: false, headerFolder: 'INBOX', folder: 'Projects' };

describe('resolveEffectiveFolder', () => {
  it('uses the header folder for a PST even though no folder mode is set', () => {
    expect(resolveEffectiveFolder({ ...base, isPst: true, folderMode: '' })).toBe('INBOX');
  });

  it('uses the detected header folder in header mode', () => {
    expect(resolveEffectiveFolder({ ...base, headerFolder: 'Receipts', folderMode: 'header' })).toBe('Receipts');
  });

  it.each(['existing', 'custom'] as const)('uses the chosen folder in %s mode', (folderMode) => {
    expect(resolveEffectiveFolder({ ...base, folderMode })).toBe('Projects');
  });

  it('is empty when no mode is chosen for EML or MBOX', () => {
    expect(resolveEffectiveFolder({ ...base, folderMode: '' })).toBe('');
  });
});
