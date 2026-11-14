/** A map that forgets its least recently used entries past `maxEntries` or `maxBytes`. */
export class Lru<V> {
  private map = new Map<string, { value: V; bytes: number }>();
  private bytes = 0;
  constructor(
    private readonly maxEntries: number,
    private readonly maxBytes = Infinity,
  ) {}

  get(key: string): V | undefined {
    const e = this.map.get(key);
    if (!e) return undefined;
    this.map.delete(key);
    this.map.set(key, e);
    return e.value;
  }

  has(key: string): boolean {
    return this.map.has(key);
  }

  set(key: string, value: V, bytes = 0): void {
    this.delete(key);
    if (bytes > this.maxBytes) return;
    this.map.set(key, { value, bytes });
    this.bytes += bytes;
    for (const [k, e] of this.map) {
      if (this.map.size <= this.maxEntries && this.bytes <= this.maxBytes) break;
      this.map.delete(k);
      this.bytes -= e.bytes;
    }
  }

  delete(key: string): void {
    const e = this.map.get(key);
    if (!e) return;
    this.map.delete(key);
    this.bytes -= e.bytes;
  }

  get size(): number {
    return this.map.size;
  }

  values(): IterableIterator<{ value: V; bytes: number }> {
    return this.map.values();
  }
}
