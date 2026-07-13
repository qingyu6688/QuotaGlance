import { lazy, Suspense } from "react";
import "./App.css";
import { isTauriRuntime } from "./api";
import { QuotaWidget } from "./components/QuotaWidget";

const PreviewBoard = import.meta.env.DEV
  ? lazy(() =>
      import("./components/PreviewBoard").then((module) => ({
        default: module.PreviewBoard,
      })),
    )
  : null;

function App() {
  const previewMode = new URLSearchParams(window.location.search).get("preview");
  if (!isTauriRuntime && previewMode === "board" && PreviewBoard !== null) {
    return (
      <Suspense fallback={<main className="quota-app preview-board">正在载入界面预览…</main>}>
        <PreviewBoard />
      </Suspense>
    );
  }

  return <QuotaWidget />;
}

export default App;
