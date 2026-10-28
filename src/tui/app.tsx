import { useApp, useInput, usePaste, useStdin, useStdout, useWindowSize } from "ink";
import { useCallback, useEffect, useRef, useState } from "react";
import { loadBoard } from "../data/client.ts";
import { BOARDS, type BoardId } from "../model.ts";
import {
  BODY_TOP,
  bodyHeightFor,
  clean,
  detailPaneWidth,
  listGeometry,
  metricPositions,
  tabBar,
  tableLayout,
} from "./layout.ts";
import { detailLines, helpLines, Screen, SPINNER } from "./screen.tsx";
import {
  type BoardState,
  type BoardStates,
  changeSort,
  initialStates,
  type MouseEvent,
  mouseDecoder,
  rowsFor,
  selectedIndex,
  selectRow,
  windowStart,
} from "./state.ts";

interface AppProps {
  initialData?: BoardStates;
  initialBoard?: number;
  mouse?: boolean;
  demo?: boolean;
}
export function App({ initialData, initialBoard = 0, mouse = true, demo = false }: AppProps) {
  const [states, setStates] = useState(initialData ?? initialStates);
  const [active, setActive] = useState(initialBoard);
  const [editing, setEditing] = useState(false);
  const [compact, setCompact] = useState(false);
  const [panel, setPanel] = useState<"help" | "details">();
  const [panelOffset, setPanelOffset] = useState(0);
  const [message, setMessage] = useState("");
  const [frame, setFrame] = useState(0);
  const board = BOARDS[active] ?? BOARDS[0];
  const { columns: width, rows: height } = useWindowSize();
  const { exit } = useApp();
  const { stdin } = useStdin();
  const { stdout } = useStdout();
  const requests = useRef(new Map<BoardId, AbortController>());
  const latest = useRef<BoardStates>(states);
  latest.current = states;
  const messageTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const patch = useCallback(
    (id: BoardId, update: (state: BoardState) => BoardState) =>
      setStates((previous) => ({ ...previous, [id]: update(previous[id]) })),
    [],
  );
  const reload = useCallback(
    async (id: BoardId) => {
      if (demo || requests.current.has(id)) return;
      const controller = new AbortController();
      requests.current.set(id, controller);
      patch(id, (state) => ({
        ...state,
        loading: true,
        error: undefined,
        selectedId: state.selectedId ?? rowsFor(state)[0]?.id,
      }));
      try {
        const result = await loadBoard(id, controller.signal);
        if (!controller.signal.aborted)
          patch(id, (state) => ({
            ...state,
            ...result,
            warning: result.warning,
            loading: false,
            error: undefined,
            updated: Date.now(),
          }));
      } catch (error) {
        if (!controller.signal.aborted)
          patch(id, (state) => ({
            ...state,
            loading: false,
            error: error instanceof Error ? error.message : String(error),
          }));
      } finally {
        requests.current.delete(id);
      }
    },
    [demo, patch],
  );
  useEffect(() => {
    for (const board of BOARDS) void reload(board.id);
    return () => {
      for (const controller of requests.current.values()) controller.abort();
      clearTimeout(messageTimer.current);
    };
  }, [reload]);
  const loading = Object.values(states).some((state) => state.loading);
  useEffect(() => {
    if (!loading || demo) return;
    const timer = setInterval(() => setFrame((frame) => (frame + 1) % SPINNER.length), 100);
    return () => clearInterval(timer);
  }, [loading, demo]);
  const notice = (value: string) => {
    setMessage(value);
    clearTimeout(messageTimer.current);
    messageTimer.current = setTimeout(() => setMessage(""), 2200);
  };
  const switchBoard = (index: number) => {
    const next = (index + BOARDS.length) % BOARDS.length;
    setActive(next);
    setPanel(undefined);
    setPanelOffset(0);
    setEditing(false);
    const board = BOARDS[next];
    if (board && latest.current[board.id].error) void reload(board.id);
  };
  const state = board ? states[board.id] : states.aa;
  const rows = rowsFor(state);
  const selected = selectedIndex(state, rows);
  const bodyHeight = bodyHeightFor(height);
  const bodyBottom = BODY_TOP + bodyHeight;
  const panelLength =
    panel === "help"
      ? helpLines(width - 4).length
      : board
        ? detailLines(board, state, width - 4).length
        : 0;
  const scrollPanel = (delta: number) =>
    setPanelOffset((offset) =>
      Math.max(0, Math.min(Math.max(0, panelLength - bodyHeight), offset + delta)),
    );
  useEffect(() => {
    setPanelOffset((offset) => Math.max(0, Math.min(offset, panelLength - bodyHeight)));
  }, [panelLength, bodyHeight]);
  const move = (delta: number) => {
    if (board) patch(board.id, (state) => selectRow(state, selectedIndex(state) + delta));
  };
  const setFilter = (filter: string) => {
    if (board)
      patch(board.id, (state) => ({
        ...state,
        filter: clean(filter),
        selectedId: undefined,
      }));
  };
  const copy = (url: boolean) => {
    const row = rows[selected];
    const value = url || board?.id === "resets" ? row?.url : row?.name;
    if (!value) return notice("No value to copy");
    stdout.write(`\x1b]52;c;${Buffer.from(value).toString("base64")}\x07`);
    notice(url ? "URL copied via OSC 52" : "Copied via OSC 52");
  };
  usePaste(
    (value) => {
      if (editing) setFilter(state.filter + value);
    },
    { isActive: editing },
  );
  useInput((input, key) => {
    if (key.ctrl && input === "c") {
      exit();
      return;
    }
    if (!board) return;
    if (editing) {
      if (key.return || key.escape) setEditing(false);
      else if (key.ctrl && input === "u") setFilter("");
      else if (key.backspace || key.delete)
        setFilter(Array.from(state.filter).slice(0, -1).join(""));
      else if (!key.ctrl && !key.meta && input) setFilter(state.filter + input);
      return;
    }
    if (input === "q") {
      exit();
      return;
    }
    if (key.escape) {
      if (panel) {
        setPanel(undefined);
        setPanelOffset(0);
      } else exit();
      return;
    }
    if (input === "?" || input === "h") {
      setPanel(panel === "help" ? undefined : "help");
      setPanelOffset(0);
      return;
    }
    if (panel) {
      if (key.return && panel === "details") setPanel(undefined);
      else if (input === "j" || key.downArrow) scrollPanel(1);
      else if (input === "k" || key.upArrow) scrollPanel(-1);
      else if (key.pageDown) scrollPanel(10);
      else if (key.pageUp) scrollPanel(-10);
      else if (input === "g" || key.home) setPanelOffset(0);
      else if (input === "G" || key.end) setPanelOffset(Math.max(0, panelLength - bodyHeight));
      else if (input === "y" || input === "Y") copy(input === "Y");
      return;
    }
    if (/^[1-7]$/.test(input)) switchBoard(Number(input) - 1);
    else if (input === "]" || (key.tab && !key.shift)) switchBoard(active + 1);
    else if (input === "[" || (key.tab && key.shift)) switchBoard(active - 1);
    else if (input === "/") setEditing(true);
    else if (input === "r") {
      if (demo) notice("Fixture preview: no network requests");
      else void reload(board.id);
    } else if (input === "z") setCompact(!compact);
    else if (input === "m" && board.chart)
      patch(board.id, (s) => ({ ...s, view: s.view === "table" ? "chart" : "table" }));
    else if (key.return) {
      setPanel("details");
      setPanelOffset(0);
    } else if (input === "j" || key.downArrow) move(1);
    else if (input === "k" || key.upArrow) move(-1);
    else if (key.pageDown) move(10);
    else if (key.pageUp) move(-10);
    else if (input === "g" || key.home) patch(board.id, (s) => selectRow(s, 0));
    else if (input === "G" || key.end) patch(board.id, (s) => selectRow(s, rowsFor(s).length - 1));
    else if (input === "o")
      patch(board.id, (s) => ({
        ...s,
        ascending: !s.ascending,
        selectedId: s.selectedId ?? rowsFor(s)[0]?.id,
      }));
    else if (input === "y" || input === "Y") copy(input === "Y");
    else {
      const column = input
        ? board.columns.find((column) => column.hotkey && column.hotkey === input)
        : undefined;
      if (column) patch(board.id, (s) => changeSort(s, column.key, column.ascending ?? false));
    }
  });
  const mouseHandler = useRef<(event: MouseEvent) => void>(() => undefined);
  mouseHandler.current = (event) => {
    if (!board || editing || width < 36 || height < 8 || event.released) return;
    if (event.button === 64 || event.button === 65) {
      if (panel) scrollPanel(event.button === 64 ? -1 : 1);
      else move(event.button === 64 ? -1 : 1);
      return;
    }
    if ((event.button & 3) !== 0) return;
    if (panel) {
      if (
        event.x === width - 2 &&
        event.y >= BODY_TOP &&
        event.y < bodyBottom &&
        panelLength > bodyHeight
      )
        setPanelOffset(
          Math.round(
            ((event.y - BODY_TOP) / Math.max(1, bodyHeight - 1)) * (panelLength - bodyHeight),
          ),
        );
      return;
    }
    if (event.y === 0) {
      const item = tabBar(width, active, BOARDS).items.find(
        (item) => event.x >= item.x && event.x < item.x + Bun.stringWidth(item.text),
      );
      if (item) switchBoard(item.index);
      return;
    }
    const mainWidth = width - detailPaneWidth(width, height);
    const list = state.view === "table" || (board.chart && board.id !== "resets");
    if (event.x >= mainWidth || event.y < BODY_TOP || event.y >= bodyBottom || !list) return;
    const { header, capacity } = listGeometry(bodyHeight);
    const rowTop = BODY_TOP + (header ? 1 : 0);
    if (
      state.view === "table" &&
      event.x === mainWidth - 2 &&
      event.y >= rowTop &&
      event.y < rowTop + capacity &&
      rows.length > capacity
    ) {
      patch(board.id, (s) =>
        selectRow(
          s,
          Math.round(((event.y - rowTop) / Math.max(1, capacity - 1)) * (rows.length - 1)),
        ),
      );
      return;
    }
    if (header && event.y === BODY_TOP) {
      if (state.view !== "table") return;
      const layout = tableLayout(board, mainWidth - 3, state.sort, compact);
      const hit = metricPositions(layout).find(
        ({ column, x }) => event.x >= 1 + x && event.x < 1 + x + column.width,
      );
      if (hit) patch(board.id, (s) => changeSort(s, hit.column.key, hit.column.ascending ?? false));
    } else {
      const index = windowStart(selected, rows.length, capacity) + event.y - rowTop;
      if (index < rows.length) patch(board.id, (s) => selectRow(s, index));
    }
  };
  useEffect(() => {
    if (!mouse || !(stdout as NodeJS.WriteStream).isTTY) return;
    const decode = mouseDecoder();
    const onData = (data: Buffer | string) => {
      for (const event of decode(data.toString())) mouseHandler.current(event);
    };
    stdout.write("\x1b[?1000h\x1b[?1002h\x1b[?1006h");
    stdin.on("data", onData);
    return () => {
      stdin.off("data", onData);
      stdout.write("\x1b[?1006l\x1b[?1002l\x1b[?1000l");
    };
  }, [mouse, stdin, stdout]);
  return (
    <Screen
      width={width}
      height={height}
      active={active}
      states={states}
      compact={compact}
      editing={editing}
      panel={panel}
      panelOffset={panelOffset}
      message={message}
      demo={demo}
      frame={frame}
    />
  );
}
