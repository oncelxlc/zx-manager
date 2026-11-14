import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Provider } from "react-redux";
import { HashRouter } from "react-router";
import App from "./App";
import { createAppStore } from "./store";
import { mainWindowSession, restoreMainWindow } from "./mainWindowSession";

const root = document.getElementById("root");
if (!root) throw new Error("Missing application root");

async function start(element: HTMLElement) {
  await restoreMainWindow().catch(error => console.error("无法恢复主窗口状态", error));
  const store = createAppStore(mainWindowSession.dependenciesExpanded);
  store.subscribe(() => {
    mainWindowSession.dependenciesExpanded = store.getState().welcome.dependenciesExpanded;
  });
  createRoot(element).render(
    <StrictMode>
      <Provider store={store}>
        <HashRouter>
          <App />
        </HashRouter>
      </Provider>
    </StrictMode>,
  );
}

void start(root);
