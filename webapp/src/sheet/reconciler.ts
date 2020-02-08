/// <reference path="./intrinsic_elements.d.ts" /> #

import { Render as _Render, Barline } from "../../rust_render_built/index";
import { unstable_now as now } from "scheduler";
import ReactReconciler from "react-reconciler";

interface RenderExtra {
  classNames: { [key: string]: string };
  boundingClassNames: { [key: string]: string };
}

type Render = _Render & RenderExtra;

export { Render, Barline } from "../../rust_render_built/index";

export enum NoteValue {
  Maxima = 3,
  Longa = 2,
  DoubleWhole = 1,
  Whole = 0,
  Half = -1,
  Quarter = -2,
  Eighth = -3,
  Sixteenth = -4,
  ThirtySecond = -5,
  SixtyFourth = -6,
  HundredTwentyEighth = -7,
  TwoHundredFiftySixth = -8
}

interface Instance {
  type: "song" | "staff" | "bar" | "between" | "rnc";
  container: Render;
  entity: number;
}

export interface SongProps {
  key?: string | number | null | undefined;
  freezeSpacing?: number | undefined;
  children: React.ReactNode;
  className?: any;
  boundingClassName?: any;
}

export interface StaffProps {
  key?: string | number | null | undefined;
  children: React.ReactNode;
  className?: any;
  boundingClassName?: any;
}

export interface BarProps {
  key?: string | number | null | undefined;
  numer: number;
  denom: number;
  children?: any;
  className?: any;
  boundingClassName?: any;
}

export interface BetweenBarsProps {
  key?: string | number | null | undefined;
  clef?: boolean;
  tsNum?: number;
  tsDen?: number;
  barline?: Barline | undefined;
  className?: any;
  boundingClassName?: any;
}

export interface RncProps {
  key?: string | number | null | undefined;
  noteValue: number;
  dots: number;
  startNum: number;
  startDen: number;
  isNote: boolean;
  className?: any;
  boundingClassName?: any;
}

// TODO: dedupe with JSX.IntrinsicElements
type CreateInstanceParam =
  | { type: "song"; props: SongProps }
  | { type: "staff"; props: StaffProps }
  | { type: "bar"; props: BarProps }
  | { type: "between"; props: BetweenBarsProps }
  | { type: "rnc"; props: RncProps }
  | { type: never; props: never };

function createInstance(
  spec: CreateInstanceParam,
  container: Render
): Instance | null {
  let type: "song" | "staff" | "bar" | "between" | "rnc";
  let entity;

  if (spec.type === "song") {
    type = "song";
    entity = container.song_create(
      typeof spec.props.freezeSpacing === "number"
        ? spec.props.freezeSpacing
        : undefined
    );
  } else if (spec.type === "staff") {
    type = "staff";
    entity = container.staff_create();
  } else if (spec.type === "bar") {
    (type = "bar"),
      (entity = container.bar_create(spec.props.numer, spec.props.denom));
  } else if (spec.type === "between") {
    type = "between";
    entity = container.between_bars_create(
      spec.props.barline,
      spec.props.clef || false,
      spec.props.tsNum || undefined,
      spec.props.tsDen || undefined
    );
  } else if (spec.type === "rnc") {
    type = "rnc";
    entity = container.rnc_create(
      spec.props.noteValue,
      spec.props.dots,
      spec.props.startNum,
      spec.props.startDen,
      spec.props.isNote
    );
  } else {
    throw new Error(`Invalid type in sheet music reconciler: <${spec.type} />`);
  }

  if ("className" in spec.props) {
    container.classNames[entity] = spec.props.className;
  }

  if ("boundingClassName" in spec.props) {
    container.boundingClassNames[entity] = spec.props.boundingClassName;
  }

  return { container, type, entity };
}

function appendChild(parent: Instance, child: Instance) {
  if (!parent || !child || parent.container !== child.container) {
    return;
  }

  if (parent.type === "bar") {
    parent.container.bar_insert(parent.entity, child.entity);
  } else {
    parent.container.child_append(parent.entity, child.entity);
  }
}

const Reconciler = ReactReconciler({
  supportsMutation: true,
  createInstance(type, props, container: Render) {
    // @ts-ignore
    return createInstance({ type, props }, container);
  },
  createTextInstance(
    _text,
    _rootContainerInstance: Render,
    _hostContext,
    _internalInstanceHandle
  ) {
    throw new Error("Text not supported.");
  },

  appendChildToContainer(container, child: Instance) {
    container.root_set(child.entity);
  },
  appendChild(parent: Instance, child: Instance) {
    appendChild(parent, child);
  },
  appendInitialChild(parent: Instance, child: Instance) {
    appendChild(parent, child);
  },

  removeChildFromContainer(_container: Render, child: Instance) {
    child.container.root_clear(child.entity);
  },
  removeChild(parent: Instance, child: Instance) {
    if (!parent || !child || parent.container !== child.container) {
      return;
    }

    if (parent.type === "bar") {
      child.container.bar_remove(parent.entity, child.entity);
    } else {
      child.container.child_remove(parent.entity, child.entity);
    }
  },
  insertInContainerBefore(
    _container: Render,
    _child: Instance,
    _before: Instance
  ) {
    throw new Error("The root can only have one child");
  },
  insertBefore(parent: Instance, child: Instance, before: Instance) {
    if (parent.type === "bar") {
      parent.container.bar_insert(parent.entity, child.entity);
    } else {
      parent.container.child_insert_before(
        parent.entity,
        before.entity,
        child.entity
      );
    }
  },

  prepareUpdate(
    _instance: Instance,
    _type,
    _oldProps: any,
    _newProps: any,
    _rootContainerInstance: Render,
    _currentHostContext
  ) {
    return {};
  },
  commitUpdate(
    instance: Instance,
    _updatePayload: any,
    type,
    oldProps: any,
    newProps: any,
    _finishedWork
  ) {
    if (type === "song" && oldProps.freezeSpacing !== newProps.freezeSpacing) {
      instance.container.song_set_freeze_spacing(
        instance.entity,
        newProps.freezeSpacing
      );
    }

    if (
      type === "rnc" &&
      (oldProps.startNum !== newProps.startNum ||
        oldProps.startDen !== newProps.startDen ||
        oldProps.noteValue !== newProps.noteValue ||
        oldProps.dots !== newProps.dots)
    ) {
      instance.container.rnc_update_time(
        instance.entity,
        newProps.noteValue,
        newProps.dots,
        newProps.startNum,
        newProps.startDen
      );
    }

    if (
      type === "between" &&
      (oldProps.clef !== newProps.clef ||
        oldProps.tsNum !== newProps.tsNum ||
        oldProps.tsDen !== newProps.tsDen)
    ) {
      instance.container.between_bars_update(
        instance.entity,
        newProps.barline,
        newProps.clef,
        newProps.tsNum,
        newProps.tsDen
      );
    }

    if (oldProps.className !== newProps.className) {
      instance.container.classNames[instance.entity] = newProps.className;
    }

    if (oldProps.boundingClassName !== newProps.boundingClassName) {
      instance.container.boundingClassNames[instance.entity] =
        newProps.boundingClassName;
    }
  },

  finalizeInitialChildren() {
    return false;
  },
  getChildHostContext() {},
  getPublicInstance(x) {
    return x;
  },
  getRootHostContext() {},
  prepareForCommit() {},
  resetAfterCommit() {},
  shouldSetTextContent() {
    return false;
  },

  now,
  setTimeout,
  clearTimeout,
  shouldDeprioritizeSubtree() {
    return false;
  },
  noTimeout: -1,
  supportsHydration: false,
  supportsPersistence: false,
  isPrimaryRenderer: false,
  cancelDeferredCallback() {},
  scheduleDeferredCallback() {
    return false;
  }
});

const roots = new Map<Render, ReactReconciler.FiberRoot>();

export function render(whatToRender: any, container: Render) {
  let root = roots.get(container);
  if (!root) {
    root = Reconciler.createContainer(container, false, false);
    roots.set(container, root);
  }

  Reconciler.updateContainer(whatToRender, root, null, () => null);
}

Reconciler.injectIntoDevTools({
  bundleType: process.env.NODE_ENV === "production" ? 0 : 1,
  version: "0.10.0",
  rendererPackageName: "six-eight",
  // @ts-ignore
  findFiberByHostInstance() {
    return null;
  }
});