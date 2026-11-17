import type { WorkerPose } from "./worker-animator";
import { canvasPixelRatio } from "@/render/use-canvas";

/**
 * The worker, drawn as vector parts in a unit frame: 1 unit = 1 cell, origin at the centre of the
 * soles, y up is negative, facing right (left is the same drawing mirrored, so eyes, brim, boots
 * and a push all turn with it). Squash scales around the soles. Each part gets a dark outline
 * first so the worker stands out on orange crates at any seat colour.
 */

const OUTLINE = "#11151d";
const SKIN = "#f2c29b";
const PANTS = "#2c3e66";
const BOOTS = "#5a3a22";
const BELT = "#1f2937";
const REFLECTIVE = "#f8fafc";
const EYES = "#11151d";
const HIGHLIGHT = "rgb(255 255 255 / 0.45)";
const SHADOW = "rgb(0 0 0 / 0.28)";

/** Leg stride (-1 back .. 1 forward for the front leg) and body lift of the 4-frame walk cycle. */
const WALK_FRAMES: readonly { readonly stride: number; readonly lift: number }[] = [
  { stride: 1, lift: 0 },
  { stride: 0, lift: 0.05 },
  { stride: -1, lift: 0 },
  { stride: 0, lift: 0.05 },
];
const STANDING = { stride: 0, lift: 0 };

const HIP_Y = -0.6;
const SHOULDER_Y = -1.03;
const LEG_WIDTH = 0.15;
const ARM_WIDTH = 0.12;
const HAND_RADIUS = 0.065;
const HEAD_X = 0.04;
const HEAD_Y = -1.33;
const HEAD_RADIUS = 0.21;

type Pass = "outline" | "fill";
const PASSES: readonly Pass[] = ["outline", "fill"];
const LEG_SIDES: readonly boolean[] = [false, true];

interface Point {
  x: number;
  y: number;
}

/** Per-draw state shared by the part painters; one scratch instance, nothing allocated per frame. */
interface Rig {
  pass: Pass;
  /** Outline width in units. */
  outline: number;
  /** Upper body offset: walk lift and breathing. */
  bodyY: number;
  /** Forward lean of the upper body while pushing. */
  lean: number;
  stride: number;
  readonly frontHand: Point;
  readonly backHand: Point;
}
const rig: Rig = {
  pass: "outline",
  outline: 0,
  bodyY: 0,
  lean: 0,
  stride: 0,
  frontHand: { x: 0, y: 0 },
  backHand: { x: 0, y: 0 },
};

/** Rounds to the canvas's pixel grid so the sprite does not shimmer as it moves. */
function snap(value: number): number {
  const ratio = canvasPixelRatio();
  return Math.round(value * ratio) / ratio;
}

export function drawWorkerSprite(
  context: CanvasRenderingContext2D,
  pose: WorkerPose,
  footX: number,
  footY: number,
  cellSize: number,
): void {
  context.save();
  context.globalAlpha *= pose.alpha;
  context.translate(snap(footX), snap(footY));
  if (!pose.airborne && pose.scaleY > 0.5) drawShadow(context, cellSize * pose.scaleX);
  context.scale(pose.facing * pose.scaleX * cellSize, pose.scaleY * cellSize);
  context.lineCap = "round";
  context.lineJoin = "round";

  const walk = pose.walkFrame >= 0 ? (WALK_FRAMES[pose.walkFrame] ?? STANDING) : STANDING;
  rig.outline = Math.max(1, cellSize * 0.05) / cellSize;
  rig.bodyY = -walk.lift + pose.breath * 0.015;
  rig.lean = pose.pushing ? 0.05 : 0;
  rig.stride = walk.stride;
  placeHands(pose);

  for (const part of PARTS_BACK_TO_FRONT) {
    for (const pass of PASSES) {
      rig.pass = pass;
      part(context, pose);
    }
  }
  context.restore();
}

/** A small white arrow above the viewer's own worker when several workers share the field. */
export function drawYouMarker(
  context: CanvasRenderingContext2D,
  x: number,
  topY: number,
  cellSize: number,
): void {
  const half = cellSize * 0.18;
  const tipY = snap(topY - cellSize * 0.12);
  context.fillStyle = "#ffffff";
  context.strokeStyle = OUTLINE;
  context.lineWidth = Math.max(1, cellSize * 0.04);
  context.beginPath();
  context.moveTo(snap(x - half), tipY - half * 1.1);
  context.lineTo(snap(x + half), tipY - half * 1.1);
  context.lineTo(snap(x), tipY);
  context.closePath();
  context.stroke();
  context.fill();
}

function drawShadow(context: CanvasRenderingContext2D, width: number): void {
  context.fillStyle = SHADOW;
  context.beginPath();
  context.ellipse(0, 0, width * 0.32, Math.max(1, width * 0.06), 0, 0, Math.PI * 2);
  context.fill();
}

function placeHands(pose: WorkerPose): void {
  const { bodyY, stride, frontHand, backHand } = rig;
  if (pose.pushing) {
    set(frontHand, 0.36, -0.9 + bodyY);
    set(backHand, 0.33, -0.8 + bodyY);
  } else if (pose.airborne) {
    // Arms out for balance; raised hands would cover the face.
    set(frontHand, 0.3, -0.98 + bodyY);
    set(backHand, -0.27, -1.04 + bodyY);
  } else {
    // Arms swing against the legs.
    set(frontHand, 0.03 - stride * 0.15, -0.68 + bodyY);
    set(backHand, -0.03 + stride * 0.15, -0.7 + bodyY);
  }
}

function set(point: Point, x: number, y: number): void {
  point.x = x;
  point.y = y;
}

/** Fills the current path, or in the outline pass strokes it so the outline shows around the fill. */
function paint(context: CanvasRenderingContext2D, color: string): void {
  if (rig.pass === "outline") {
    context.strokeStyle = OUTLINE;
    context.lineWidth = rig.outline * 2;
    context.stroke();
    return;
  }
  context.fillStyle = color;
  context.fill();
}

/** Strokes the current path as a limb `width` wide, outlined in the outline pass. */
function stroke(context: CanvasRenderingContext2D, width: number, color: string): void {
  const outlined = rig.pass === "outline";
  context.strokeStyle = outlined ? OUTLINE : color;
  context.lineWidth = outlined ? width + rig.outline * 2 : width;
  context.stroke();
}

function drawArm(context: CanvasRenderingContext2D, pose: WorkerPose, hand: Point): void {
  context.beginPath();
  context.moveTo(rig.lean, SHOULDER_Y + rig.bodyY);
  context.lineTo(hand.x, hand.y);
  stroke(context, ARM_WIDTH, pose.shade);
  context.beginPath();
  context.arc(hand.x, hand.y, HAND_RADIUS, 0, Math.PI * 2);
  paint(context, SKIN);
}

function drawLegs(context: CanvasRenderingContext2D, pose: WorkerPose): void {
  const hipY = HIP_Y + rig.bodyY;
  for (const front of LEG_SIDES) {
    const hipX = front ? 0.06 : -0.06;
    let footX = hipX + (front ? rig.stride : -rig.stride) * 0.14;
    let footY = 0;
    if (pose.airborne) {
      footX = front ? 0.16 : -0.1;
      footY = front ? -0.16 : -0.08;
    }
    context.beginPath();
    context.moveTo(hipX, hipY);
    context.lineTo(footX, footY - 0.1);
    stroke(context, LEG_WIDTH, PANTS);
    context.beginPath();
    context.roundRect(footX - 0.09, footY - 0.13, 0.25, 0.13, [0.04, 0.07, 0.02, 0.02]);
    paint(context, BOOTS);
  }
}

function drawTorso(context: CanvasRenderingContext2D, pose: WorkerPose): void {
  const { lean, bodyY } = rig;
  context.beginPath();
  context.roundRect(-0.22 + lean, -1.12 + bodyY, 0.44, 0.62, 0.1);
  paint(context, pose.color);
  if (rig.pass === "outline") return;
  context.fillStyle = REFLECTIVE;
  context.fillRect(-0.22 + lean, -0.88 + bodyY, 0.44, 0.07);
  context.fillStyle = BELT;
  context.fillRect(-0.22 + lean, -0.6 + bodyY, 0.44, 0.07);
}

function drawHead(context: CanvasRenderingContext2D, pose: WorkerPose): void {
  const x = HEAD_X + rig.lean;
  const y = HEAD_Y + rig.bodyY;
  context.beginPath();
  context.arc(x, y, HEAD_RADIUS, 0, Math.PI * 2);
  paint(context, SKIN);
  if (rig.pass === "fill") drawFace(context, pose, x, y);

  // Hard hat: a dome with a brim that sticks out in front.
  context.beginPath();
  context.arc(x - 0.01, y - 0.07, 0.235, Math.PI, 0);
  context.closePath();
  paint(context, pose.color);
  context.beginPath();
  context.roundRect(x - 0.25, y - 0.1, 0.55, 0.07, 0.03);
  paint(context, pose.color);
  if (rig.pass === "outline") return;
  context.strokeStyle = HIGHLIGHT;
  context.lineWidth = 0.05;
  context.beginPath();
  context.arc(x - 0.01, y - 0.07, 0.16, Math.PI * 1.15, Math.PI * 1.45);
  context.stroke();
}

function drawFace(context: CanvasRenderingContext2D, pose: WorkerPose, headX: number, headY: number): void {
  context.fillStyle = EYES;
  context.strokeStyle = EYES;
  drawEye(context, pose, headX + 0.06, headY + 0.03);
  drawEye(context, pose, headX + 0.15, headY + 0.03);
}

function drawEye(context: CanvasRenderingContext2D, pose: WorkerPose, x: number, y: number): void {
  if (pose.eyes === "open") context.fillRect(x - 0.025, y - 0.045, 0.05, 0.09);
  else if (pose.eyes === "closed") context.fillRect(x - 0.03, y - 0.01, 0.06, 0.025);
  else {
    context.lineWidth = 0.025;
    context.beginPath();
    context.moveTo(x - 0.03, y - 0.03);
    context.lineTo(x + 0.03, y + 0.03);
    context.moveTo(x + 0.03, y - 0.03);
    context.lineTo(x - 0.03, y + 0.03);
    context.stroke();
  }
}

type PartPainter = (context: CanvasRenderingContext2D, pose: WorkerPose) => void;
/** Each part is outlined, then filled, before the next one goes on top. */
const PARTS_BACK_TO_FRONT: readonly PartPainter[] = [
  (context, pose) => drawArm(context, pose, rig.backHand),
  drawLegs,
  drawTorso,
  drawHead,
  (context, pose) => drawArm(context, pose, rig.frontHand),
];
