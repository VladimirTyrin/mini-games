import { describe, expect, it } from "vitest";

import { MAX_BUFFERED_PRESSES, type WorkerAction, WorkerInputPacer } from "./worker-input-pacer";

function pacer(canSend = () => true): { pacer: WorkerInputPacer; sent: WorkerAction[] } {
  const sent: WorkerAction[] = [];
  return {
    sent,
    pacer: new WorkerInputPacer((action) => {
      if (!canSend()) return false;
      sent.push(action);
      return true;
    }),
  };
}

/** One tick of the authority: the sent command is acknowledged, then executed in the next frame. */
function tick(subject: WorkerInputPacer, queuedAfterStep = 0): void {
  if (subject.hasCommandsInFlight) subject.acknowledged();
  subject.frameArrived(queuedAfterStep);
  subject.pump();
}

describe("WorkerInputPacer", () => {
  it("sends a press immediately when nothing is in flight", () => {
    const { pacer: subject, sent } = pacer();
    subject.press("left");
    expect(sent).toEqual(["left"]);
    expect(subject.hasCommandsInFlight).toBe(true);
  });

  it("walks one step per tick while a direction is held", () => {
    const { pacer: subject, sent } = pacer();
    subject.press("right");
    tick(subject);
    tick(subject);
    tick(subject);
    expect(sent).toEqual(["right", "right", "right", "right"]);
    subject.release("right");
    tick(subject);
    tick(subject);
    expect(sent).toHaveLength(4);
  });

  it("never has more than one command in the pipeline", () => {
    const { pacer: subject, sent } = pacer();
    subject.press("right");
    subject.press("jump");
    subject.press("left");
    expect(sent).toEqual(["right"]);
    // Acknowledged but the frame containing it has not arrived yet: still in flight.
    subject.acknowledged();
    subject.pump();
    expect(sent).toEqual(["right"]);
  });

  it("waits while the latest view still shows queued commands", () => {
    const { pacer: subject, sent } = pacer();
    subject.press("right");
    tick(subject, 1);
    expect(sent).toEqual(["right"]);
    tick(subject, 0);
    expect(sent).toEqual(["right", "right"]);
  });

  it("keeps a quick tap that was released before the pipeline emptied", () => {
    const { pacer: subject, sent } = pacer();
    subject.press("jump");
    subject.press("right");
    subject.release("right");
    tick(subject);
    expect(sent).toEqual(["jump", "right"]);
    tick(subject);
    expect(sent).toEqual(["jump", "right"]);
  });

  it("buffers a jump pressed while walking before resuming the walk", () => {
    const { pacer: subject, sent } = pacer();
    subject.press("left");
    subject.press("jump");
    tick(subject);
    tick(subject);
    expect(sent).toEqual(["left", "jump", "left"]);
  });

  it(`drops the oldest presses beyond ${MAX_BUFFERED_PRESSES} buffered`, () => {
    const { pacer: subject, sent } = pacer();
    subject.press("jump");
    subject.press("left");
    subject.release("left");
    subject.press("jump");
    subject.press("right");
    subject.release("right");
    tick(subject);
    tick(subject);
    tick(subject);
    expect(sent).toEqual(["jump", "jump", "right"]);
  });

  it("the latest held direction wins and the earlier one resumes on release", () => {
    const { pacer: subject, sent } = pacer();
    subject.press("left");
    tick(subject);
    subject.press("right");
    tick(subject);
    subject.release("right");
    tick(subject);
    expect(sent).toEqual(["left", "left", "right", "left"]);
  });

  it("resumes after a refused command on the next frame", () => {
    const { pacer: subject, sent } = pacer();
    subject.press("right");
    subject.rejected();
    expect(subject.hasCommandsInFlight).toBe(false);
    subject.frameArrived(0);
    subject.pump();
    expect(sent).toEqual(["right", "right"]);
  });

  it("does not count commands that could not be sent", () => {
    let alive = false;
    const { pacer: subject, sent } = pacer(() => alive);
    subject.press("left");
    expect(subject.hasCommandsInFlight).toBe(false);
    alive = true;
    subject.reset();
    subject.press("right");
    expect(sent).toEqual(["right"]);
  });
});
