import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import { newRender, render } from "./reconciler";

export type Render = import("./reconciler").Render;
export { NoteValue, Barline } from "./reconciler";
export const TYPE_RNC = 0;
export const TYPE_BETWEEN_BARS = 1;

interface Props {
  children: any;
  onMouseDown?: (
    time: null | [number, number, number, number],
    ev: React.MouseEvent,
  ) => void;
  onMouseUp?: (
    time: null | [number, number, number, number],
    ev: React.MouseEvent,
  ) => void;
  onClick?: (
    time: null | [number, number, number, number],
    ev: React.MouseEvent,
  ) => void;
  onMouseMove?: (ev: React.MouseEvent) => void;
  onHover: (
    time: [number, number, number] | null,
    pitch: number | null,
  ) => void;
}

/** [entity, x, y, scale] */
type StencilMapItem = [number, number, number];
type StencilOrStencilMap = string | Array<StencilMapItem>;
/** [x, y, x2, y2, barIdx, timeFracNum, timeFracDen, kind] */
type StencilMeta = [
  number,
  number,
  number,
  number,
  number,
  number,
  number,
  number,
];

function StencilView({
  id,
  stencils,
  stencilMeta,
  transform,
  classNames,
}: {
  id: number;
  stencils: { [key: string]: StencilOrStencilMap };
  stencilMeta: { [key: string]: StencilMeta };
  transform?: string;
  classNames: { [key: string]: string };
}) {
  const stencil = stencils[id];
  if (typeof stencil === "string") {
    return (
      <g
        className={classNames[id] || undefined}
        transform={transform}
        data-entity-id={id}
        dangerouslySetInnerHTML={{ __html: stencil }}
      />
    );
  } else {
    return (
      <g
        transform={transform}
        data-entity-id={id}
        className={classNames[id] || undefined}
      >
        {stencil.map(([childId, x, y]) => (
          <StencilView
            key={childId}
            id={childId}
            stencils={stencils}
            stencilMeta={stencilMeta}
            classNames={classNames}
            transform={
              typeof x === "number" ? `translate(${x}, ${y})` : undefined
            }
          />
        ))}
      </g>
    );
  }
}

export default function SheetMusicView(props: Props) {
  // create/destroy Rust container
  const [container] = useState(newRender);
  useEffect(() => {
    return () => {
      container.free();
    };
  }, [container]);

  // render loop
  const [stencils, setStencils] = useState<{
    [key: number]: StencilOrStencilMap;
  } | null>(null);
  const [stencilMeta, setStencilMeta] = useState<{
    [key: number]: StencilMeta;
  } | null>(null);
  const [children, setChildren] = useState<{
    [key: number]: Array<number>;
  }>({});
  const [root, setRoot] = useState<number | null>(null);
  const [hoverInfo, setHoverInfo] = useState<
    [number, number, number, number] | null
  >(null);
  const [pageSize, setPageSize] = useState({ width: 0, height: 0 });

  useLayoutEffect(() => {
    render(props.children, container);
    container.exec();
    let stencilPairs = container.stencils().split("\n");
    let stencilMapPairs = container.stencil_maps().split("\n");
    let stencilMetaPairs = container.get_stencil_bboxes().split("\n");
    let parents = container.parents().split("\n");

    let stencils: { [key: number]: StencilOrStencilMap } = {};
    for (let i = 0; i < stencilPairs.length; i += 2) {
      stencils[stencilPairs[i] as any] = stencilPairs[i + 1];
    }

    for (let i = 0; i < stencilMapPairs.length; i += 2) {
      stencils[stencilMapPairs[i] as any] = JSON.parse(stencilMapPairs[i + 1]);
    }

    let stencilMeta: { [key: number]: StencilMeta } = {};
    for (let i = 0; i < stencilMetaPairs.length; i += 2) {
      stencilMeta[stencilMetaPairs[i] as any] = JSON.parse(
        stencilMetaPairs[i + 1],
      );
    }

    let children: { [key: number]: Array<number> } = {};
    for (let i = 0; i < parents.length; i += 2) {
      let cl = children[parents[i + 1] as any] || [];
      cl.push(parseInt(parents[i]));
      children[parents[i + 1] as any] = cl;
    }

    setStencils(stencils);
    setStencilMeta(stencilMeta);
    setChildren(children);
    const root = container.get_root_id();
    setRoot(root || null);
    setPageSize({
      width: (root && container.get_song_width(root)) || 0,
      height: (root && container.get_song_height(root)) || 0,
    });
  }, [container, props.children]);

  const svg = useRef<SVGSVGElement>(null);

  const bound = svg.current && svg.current.getBoundingClientRect();

  function makeMouseHandler(
    fn?: (
      time: null | [number, number, number, number],
      ev: React.MouseEvent,
    ) => void,
  ) {
    return (ev: React.MouseEvent) => {
      if (!stencilMeta || !fn) {
        return;
      }

      fn(hoverInfo, ev);
    };
  }

  return (
    <>
      <svg
        className="six-sheet"
        viewBox={`0 0 ${pageSize.width} ${pageSize.height}`}
        width="100%"
        ref={svg}
        onMouseDownCapture={makeMouseHandler(props.onMouseDown)}
        onMouseUpCapture={makeMouseHandler(props.onMouseUp)}
        onClick={makeMouseHandler(props.onClick)}
        onMouseMove={ev => {
          if (!svg || !svg.current || !stencilMeta) {
            return;
          }
          const ctm = svg.current.getScreenCTM();
          if (!ctm) {
            return;
          }
          let pt = svg.current.createSVGPoint();
          pt.x = ev.clientX;
          pt.y = ev.clientY;
          pt = pt.matrixTransform(ctm.inverse());

          const info = container.get_hover_info(pt.x, pt.y);

          if (info) {
            if (
              Boolean(hoverInfo) !== Boolean(info) ||
              (hoverInfo &&
                (info[0] !== hoverInfo[0] ||
                  info[1] !== hoverInfo[1] ||
                  info[2] !== hoverInfo[2] ||
                  info[3] !== hoverInfo[3]))
            ) {
              setHoverInfo(info ? [info[0], info[1], info[2], info[3]] : null);
              props.onHover(
                info ? [info[0], info[1], info[2]] : null,
                info ? info[3] : null,
              );
            }
          }

          if (props.onMouseMove) {
            props.onMouseMove(ev);
          }
        }}
      >
        {root && stencils && stencils[root] && stencilMeta && (
          <StencilView
            id={root}
            stencils={stencils}
            stencilMeta={stencilMeta}
            classNames={container.classNames}
          />
        )}
      </svg>
      {stencilMeta &&
        Object.entries(container.html).map(([id, html]) => {
          const meta = stencilMeta[id as any];
          let applyTo;
          if (meta) {
            applyTo = [meta];
          } else {
            applyTo = (children[id as any] || [])
              .map(m => stencilMeta[m])
              .filter(m => m);
          }

          return (
            <React.Fragment key={id}>
              {applyTo.map((meta, i) => {
                if (!meta || !html || !svg.current || !bound) {
                  return null;
                }

                const ctm = svg.current.getScreenCTM();
                if (!ctm) {
                  return;
                }

                let pt2 = svg.current.createSVGPoint();
                pt2.x = meta[0];
                pt2.y = meta[1];
                pt2 = pt2.matrixTransform(ctm);

                let pt3 = svg.current.createSVGPoint();
                pt3.x = meta[2];
                pt3.y = meta[3];
                pt3 = pt3.matrixTransform(ctm);

                const width = pt3.x - pt2.x;
                const height = pt3.y - pt2.y;
                return (
                  <div
                    key={i}
                    style={{
                      position: "absolute",
                      left: Math.round(pt2.x - bound.left),
                      top: Math.round(pt2.y - bound.top),
                      width,
                      height,
                    }}
                  >
                    <div style={{ position: "relative" }}>
                      {html({ width, height })}
                    </div>
                  </div>
                );
              })}
            </React.Fragment>
          );
        })}
    </>
  );
}
