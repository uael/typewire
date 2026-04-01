import { createRoot } from "react-dom/client";
import { configure } from "mobx";
import { App } from "./App.tsx";
import { initStore } from "./store.ts";

// MobX strict mode: only allow state changes inside actions
configure({ enforceActions: "observed" });

const root = createRoot(document.getElementById("root")!);

initStore().then(() => {
  root.render(<App />);
});
