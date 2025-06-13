import { ink, React } from "./deps.ts";

const { useEffect, useState } = React;
const { Box } = ink;

export type ConsoleSize = ReturnType<typeof Deno.consoleSize>;

const ConsoleSizeContext = React.createContext<ConsoleSize>(Deno.consoleSize());
export function ConsoleSizeProvider({ children }: React.PropsWithChildren) {
  function useConsoleSizeState() {
    const [size, setSize] = useState(Deno.consoleSize());
    useEffect(() => {
      const updateSize = () => {
        const newSize = Deno.consoleSize();
        if (newSize.rows !== size.rows || newSize.columns != size.columns) {
          setSize({ ...newSize });
        }
      };
      if (Deno.build.os === "windows") {
        const intervalId = setInterval(updateSize, 50);
        return () => clearTimeout(intervalId);
      } else {
        Deno.addSignalListener("SIGWINCH", updateSize);
        return () => Deno.removeSignalListener("SIGWINCH", updateSize);
      }
    });
    return size;
  }

  const size = useConsoleSizeState();
  return (
    <ConsoleSizeContext.Provider value={size}>
      {children}
    </ConsoleSizeContext.Provider>
  );
}

export function useConsoleSize() {
  const context = React.useContext(ConsoleSizeContext);
  if (!context) {
    throw new Error(
      "useConsoleSize must only be used within children of <ConsoleSizeProvider />",
    );
  }
  return context;
}

/** Ensures that the UI is never larger than the terminal's size. Ink seems to
 * have trouble clearing previously drawn text if this component isn't used to limit
 * the size of the UI. */
export function BoundToTerminalSize({ children }: React.PropsWithChildren) {
  const size = useConsoleSize();
  return (
    <Box
      flexDirection="column"
      padding={0}
      margin={0}
      width={size.columns - 1}
      height={size.rows - 1}
      overflow="hidden"
    >
      {children}
    </Box>
  );
}
