import { Navigate, Route, Routes } from "react-router";
import { WelcomePage } from "./WelcomePage";
import { IslandSettingsPage } from "./features/island/IslandSettingsPage";
import { IslandWindowPage } from "./features/island/IslandWindowPage";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<WelcomePage />} />
      <Route path="/settings/island" element={<IslandSettingsPage />} />
      <Route path="/island" element={<IslandWindowPage />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
