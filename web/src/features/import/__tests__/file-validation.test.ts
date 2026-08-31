import { describe, expect, it } from 'vitest';

import {
  MAX_EML,
  buildQueue,
  classifyFile,
  getExtension,
  isValidFileType,
  type UploadLimits,
} from '../file-validation';

/** Build a File whose reported `size` is `bytes`, without allocating that much memory. */
function fileOfSize(name: string, bytes: number, type = ''): File {
  const file = new File(['x'], name, { type });
  Object.defineProperty(file, 'size', { value: bytes });
  return file;
}

const MB = 1024 * 1024;
const limits: UploadLimits = { maxMbox: 10 * MB, maxPst: 20 * MB };

describe('getExtension', () => {
  it('returns the lower-cased extension', () => {
    expect(getExtension('inbox.MBOX')).toBe('mbox');
    expect(getExtension('a.b.eml')).toBe('eml');
  });

  it('returns the whole name when there is no dot', () => {
    expect(getExtension('README')).toBe('readme');
  });
});

describe('isValidFileType', () => {
  it('accepts allowed extensions with no MIME type', () => {
    for (const ext of ['eml', 'mbox', 'pst']) {
      expect(isValidFileType(new File([''], `f.${ext}`), ext)).toBe(true);
    }
  });

  it('rejects disallowed extensions', () => {
    expect(isValidFileType(new File([''], 'f.txt'), 'txt')).toBe(false);
  });

  it('rejects blocked MIME types even with an allowed extension', () => {
    const blocked = ['image/png', 'video/mp4', 'application/pdf', 'application/zip'];
    for (const type of blocked) {
      expect(isValidFileType(new File([''], 'f.eml', { type }), 'eml')).toBe(false);
    }
  });
});

describe('classifyFile', () => {
  it('validates an mbox against the mbox limit', () => {
    expect(classifyFile(fileOfSize('a.mbox', 5 * MB), limits).sizeOk).toBe(true);
    expect(classifyFile(fileOfSize('a.mbox', 11 * MB), limits).sizeOk).toBe(false);
  });

  it('validates a pst against the pst limit', () => {
    expect(classifyFile(fileOfSize('a.pst', 15 * MB), limits).sizeOk).toBe(true);
    expect(classifyFile(fileOfSize('a.pst', 21 * MB), limits).sizeOk).toBe(false);
  });

  it('validates an eml against the fixed EML limit', () => {
    expect(classifyFile(fileOfSize('a.eml', MAX_EML), limits).sizeOk).toBe(true);
    expect(classifyFile(fileOfSize('a.eml', MAX_EML + 1), limits).sizeOk).toBe(false);
  });

  it('treats the limit as inclusive', () => {
    expect(classifyFile(fileOfSize('a.mbox', 10 * MB), limits).sizeOk).toBe(true);
  });

  it('uses the passed limits, not a captured default', () => {
    const file = fileOfSize('a.mbox', 50 * MB);
    expect(classifyFile(file, { maxMbox: 100 * MB, maxPst: 20 * MB }).sizeOk).toBe(true);
    expect(classifyFile(file, { maxMbox: 10 * MB, maxPst: 20 * MB }).sizeOk).toBe(false);
  });

  it('flags an invalid type independently of size', () => {
    const q = classifyFile(fileOfSize('a.txt', 1), limits);
    expect(q.typeOk).toBe(false);
    expect(q.sizeOk).toBe(true);
  });
});

describe('buildQueue', () => {
  it('classifies each file and preserves order', () => {
    const queue = buildQueue(
      [fileOfSize('one.mbox', 5 * MB), fileOfSize('two.mbox', 11 * MB)],
      limits,
    );
    expect(queue.map((q) => q.file.name)).toEqual(['one.mbox', 'two.mbox']);
    expect(queue.map((q) => q.sizeOk)).toEqual([true, false]);
  });
});
