import { Navigate, Route, Routes } from "react-router";
import { WelcomePage } from "./WelcomePage";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<WelcomePage />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
