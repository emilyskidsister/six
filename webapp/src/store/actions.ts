import { Bar, Clef, State, TiedNote } from "./state";

export interface AddNote {
  type: "ADD_NOTE";
  barIdx: number;
  startTime: [number, number];
  divisions: TiedNote;
}
export function addNote(insertion: Omit<AddNote, "type">): AddNote {
  return {
    type: "ADD_NOTE",
    ...insertion,
  };
}

export interface RemoveNote {
  type: "REMOVE_NOTE";
  barIdx: number;
  startTime: [number, number];
  divisions: TiedNote;
}
export function removeNote(insertion: Omit<AddNote, "type">): RemoveNote {
  return {
    type: "REMOVE_NOTE",
    ...insertion,
  };
}

export interface SetTs {
  type: "SET_TS";
  beforeBar: number;
  ts: [number, number];
  prevTs: [number, number];

  // We never change the ts of existing music.
  // Instead, we add bars when one is already filled.
  barAddCount: number;
  barKeepCount: number;
  // For undo.
  barRemoveCount: number;

  after: [number, number] | null;
}
export function setTs(
  appState: State,
  { ts, beforeBar }: { ts: [number, number]; beforeBar: number },
): SetTs {
  let barsWithoutContent = 0;
  let prevTs = appState.song.global.between[0].ts;
  for (let i = 0; i < beforeBar; i += 1) {
    let ts = appState.song.global.between[i]?.ts;
    if (ts) {
      prevTs = ts;
    }
  }

  let after = prevTs;
  while (
    appState.song.part.bars[barsWithoutContent + beforeBar]?.notes.length ===
      0 &&
    (!barsWithoutContent ||
      !appState.song.global.between[barsWithoutContent + beforeBar]?.ts)
  ) {
    barsWithoutContent += 1;
    after =
      appState.song.global.between[barsWithoutContent + beforeBar]?.ts ?? after;
  }

  return {
    type: "SET_TS",
    beforeBar,
    ts,
    prevTs,
    barAddCount: barsWithoutContent === 0 ? 1 : 0,
    barKeepCount: barsWithoutContent,
    barRemoveCount: 0,
    after,
  };
}

export interface SetKs {
  type: "SET_KS";
  ks?: number;
  prevKs?: number;
  beforeBar: number;
}
export function setKs(
  appState: State,
  { ks, beforeBar }: { ks: number; beforeBar: number },
): SetKs {
  return {
    type: "SET_KS",
    ks,
    prevKs: appState.song.global.between[0].ks,
    beforeBar,
  };
}

export interface SetClef {
  type: "SET_CLEF";
  clef?: Clef;
  prevClef?: Clef;
  beforeBar: number;
}
export function setClef(
  appState: State,
  { clef, beforeBar }: { clef: Clef; beforeBar: number },
): SetClef {
  let priorClef: Clef | undefined;
  for (let i = 0; i < beforeBar; i += 1) {
    priorClef = appState.song.global.between[i]?.clef;
  }

  return {
    type: "SET_CLEF",
    clef: priorClef === clef ? undefined : clef,
    prevClef: appState.song.global.between[beforeBar]?.clef ?? undefined,
    beforeBar,
  };
}

export interface AddBar {
  type: "ADD_BAR";
  barIdx: number;
  bar: Bar;
}
export function addBar({ barIdx, bar }: { barIdx: number; bar: Bar }): AddBar {
  return {
    type: "ADD_BAR",
    barIdx,
    bar,
  };
}

export interface RemoveBar {
  type: "REMOVE_BAR";
  barIdx: number;
  bar: Bar;
}
export function removeBar({
  barIdx,
  bar,
}: {
  barIdx: number;
  bar: Bar;
}): RemoveBar {
  return {
    type: "REMOVE_BAR",
    barIdx,
    bar,
  };
}

export interface SetBarCount {
  type: "SET_BAR_COUNT";
  count: number;
  prevCount: number;
}
export function setBarCount(appState: State, count: number): SetBarCount {
  return {
    type: "SET_BAR_COUNT",
    count,
    prevCount: appState.song.part.bars.length,
  };
}

export interface SetTitle {
  type: "SET_TITLE";
  title: string;
  prevTitle: string;
}
export function setTitle(appState: State, title: string): SetTitle {
  return {
    type: "SET_TITLE",
    title,
    prevTitle: appState.song.global.title,
  };
}

export interface SetAuthor {
  type: "SET_AUTHOR";
  author: string;
  prevAuthor: string;
}
export function setAuthor(appState: State, author: string): SetAuthor {
  return {
    type: "SET_AUTHOR",
    author,
    prevAuthor: appState.song.global.author,
  };
}

export interface Undo {
  type: "UNDO";
}
export function undo(): Undo {
  return { type: "UNDO" };
}

export interface Redo {
  type: "REDO";
}
export function redo(): Redo {
  return { type: "REDO" };
}

export interface Reset {
  type: "RESET";
}
export function reset(): Reset {
  return { type: "RESET" };
}

export type Invertible =
  | AddNote
  | RemoveNote
  | SetKs
  | SetTs
  | SetClef
  | AddBar
  | RemoveBar
  | SetBarCount
  | SetTitle
  | SetAuthor;

export type Action = Invertible | Undo | Redo | Reset;
