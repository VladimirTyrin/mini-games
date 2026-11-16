/** Maps world units (whatever the board uses, e.g. cells) to canvas CSS pixels. */
export interface ParticleProjection {
  /** Pixels per world unit. */
  readonly scale: number;
  pixelX(worldX: number): number;
  pixelY(worldY: number): number;
}

export interface ParticleBurst {
  readonly x: number;
  readonly y: number;
  readonly count: number;
  /** World units per second. */
  readonly speed: number;
  /** Direction of the burst in radians (0 = +x, π/2 = +y) and how far particles fan out around it. */
  readonly angle: number;
  readonly spread: number;
  /** World units. */
  readonly size: number;
  readonly lifeMs: number;
  readonly color: string;
  /** `performance.now()` when the burst happens; may lie in the future (bursts scheduled ahead). */
  readonly at: number;
}

/**
 * Cosmetic particles in a fixed ring of slots: emitting overwrites the oldest particle and drawing
 * evaluates the closed-form trajectory, so nothing is allocated or stepped per frame.
 */
export class ParticleField {
  private readonly startX: Float64Array;
  private readonly startY: Float64Array;
  private readonly velocityX: Float64Array;
  private readonly velocityY: Float64Array;
  private readonly bornAt: Float64Array;
  private readonly lifeMs: Float64Array;
  private readonly sizes: Float64Array;
  private readonly colors: string[];
  private next = 0;

  /** @param gravity acceleration along world y in units/s² (negative when world y grows upwards). */
  constructor(
    readonly capacity: number,
    private readonly gravity: number,
    private readonly random: () => number = Math.random,
  ) {
    this.startX = new Float64Array(capacity);
    this.startY = new Float64Array(capacity);
    this.velocityX = new Float64Array(capacity);
    this.velocityY = new Float64Array(capacity);
    this.bornAt = new Float64Array(capacity);
    this.lifeMs = new Float64Array(capacity);
    this.sizes = new Float64Array(capacity);
    this.colors = new Array<string>(capacity).fill("");
  }

  emit(burst: ParticleBurst): void {
    for (let particle = 0; particle < burst.count; particle++) {
      const slot = this.next;
      this.next = (this.next + 1) % this.capacity;
      const angle = burst.angle + (this.random() - 0.5) * burst.spread;
      const speed = burst.speed * (0.5 + this.random() * 0.5);
      this.startX[slot] = burst.x;
      this.startY[slot] = burst.y;
      this.velocityX[slot] = Math.cos(angle) * speed;
      this.velocityY[slot] = Math.sin(angle) * speed;
      this.bornAt[slot] = burst.at;
      this.lifeMs[slot] = burst.lifeMs * (0.7 + this.random() * 0.3);
      this.sizes[slot] = burst.size * (0.6 + this.random() * 0.4);
      this.colors[slot] = burst.color;
    }
  }

  clear(): void {
    this.lifeMs.fill(0);
  }

  /** Some particle is still to be shown at `now` or later. */
  isActive(now: number): boolean {
    for (let slot = 0; slot < this.capacity; slot++) {
      if ((this.lifeMs[slot] ?? 0) > 0 && now < (this.bornAt[slot] ?? 0) + (this.lifeMs[slot] ?? 0))
        return true;
    }
    return false;
  }

  /** World position of a particle `ageMs` after its birth (exposed for tests). */
  positionAt(slot: number, ageMs: number): { x: number; y: number } {
    return { x: this.xAt(slot, ageMs / 1000), y: this.yAt(slot, ageMs / 1000) };
  }

  draw(context: CanvasRenderingContext2D, projection: ParticleProjection, now: number): void {
    const alpha = context.globalAlpha;
    for (let slot = 0; slot < this.capacity; slot++) {
      const life = this.lifeMs[slot] ?? 0;
      const age = now - (this.bornAt[slot] ?? 0);
      if (life <= 0 || age < 0 || age >= life) continue;
      const size = Math.max(1, (this.sizes[slot] ?? 0) * projection.scale * (1 - age / life / 2));
      context.globalAlpha = alpha * (1 - age / life);
      context.fillStyle = this.colors[slot] ?? "";
      context.fillRect(
        projection.pixelX(this.xAt(slot, age / 1000)) - size / 2,
        projection.pixelY(this.yAt(slot, age / 1000)) - size / 2,
        size,
        size,
      );
    }
    context.globalAlpha = alpha;
  }

  private xAt(slot: number, seconds: number): number {
    return (this.startX[slot] ?? 0) + (this.velocityX[slot] ?? 0) * seconds;
  }

  private yAt(slot: number, seconds: number): number {
    return (
      (this.startY[slot] ?? 0) +
      (this.velocityY[slot] ?? 0) * seconds +
      0.5 * this.gravity * seconds * seconds
    );
  }
}
