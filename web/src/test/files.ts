/** Build a File whose reported `size` is `bytes`, without allocating that much memory. */
export function fileOfSize(name: string, bytes: number, type = ''): File {
  const file = new File(['x'], name, { type });
  Object.defineProperty(file, 'size', { value: bytes });
  return file;
}
