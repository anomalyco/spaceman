import { StrictMode } from "react";
import { renderToString } from "react-dom/server";
import { MotionConfig } from "motion/react";
import { App } from "./App";

export function render(): string {
  return renderToString(
    <StrictMode>
      <MotionConfig reducedMotion="user">
        <App />
      </MotionConfig>
    </StrictMode>,
  );
}
