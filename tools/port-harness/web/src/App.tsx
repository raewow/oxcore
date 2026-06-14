import { Routes, Route, NavLink } from "react-router-dom";
import { Dashboard } from "./pages/Dashboard";
import { Files } from "./pages/Files";
import { FileDetail } from "./pages/FileDetail";
import { Tasks } from "./pages/Tasks";
import { Flows } from "./pages/Flows";
import { FlowDetail } from "./pages/FlowDetail";
import { Features } from "./pages/Features";
import { FeatureDetail } from "./pages/FeatureDetail";
import { SymbolDetail } from "./pages/SymbolDetail";
import { Jobs } from "./pages/Jobs";
import { Discover } from "./pages/Discover";
import Settings from "./pages/Settings";

export default function App() {
  return (
    <div className="app-layout">
      <aside className="sidebar">
        <h1>Port Harness</h1>
        <nav>
          <NavLink to="/" end>
            Dashboard
          </NavLink>
          <NavLink to="/files">Files</NavLink>
          <NavLink to="/features">Features</NavLink>
          <NavLink to="/tasks">Tasks</NavLink>
          <NavLink to="/flows">Flows</NavLink>
          <NavLink to="/discover">Discover</NavLink>
          <NavLink to="/jobs">Jobs</NavLink>
          <NavLink to="/settings">Settings</NavLink>
        </nav>
      </aside>
      <main className="main-content">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/files" element={<Files />} />
          <Route path="/files/detail" element={<FileDetail />} />
          <Route path="/features" element={<Features />} />
          <Route path="/features/:id" element={<FeatureDetail />} />
          <Route path="/tasks" element={<Tasks />} />
          <Route path="/flows" element={<Flows />} />
          <Route path="/flows/:id" element={<FlowDetail />} />
          <Route path="/jobs" element={<Jobs />} />
          <Route path="/discover" element={<Discover />} />
          <Route path="/symbols/:id" element={<SymbolDetail />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </main>
    </div>
  );
}
