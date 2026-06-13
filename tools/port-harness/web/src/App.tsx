import { Routes, Route, NavLink } from "react-router-dom";
import { Dashboard } from "./pages/Dashboard";
import { Files } from "./pages/Files";
import { Tasks } from "./pages/Tasks";
import { Flows } from "./pages/Flows";
import { FlowDetail } from "./pages/FlowDetail";
import { SymbolDetail } from "./pages/SymbolDetail";
import { Jobs } from "./pages/Jobs";
import { Discover } from "./pages/Discover";

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
          <NavLink to="/tasks">Tasks</NavLink>
          <NavLink to="/flows">Flows</NavLink>
          <NavLink to="/discover">Discover</NavLink>
          <NavLink to="/jobs">Jobs</NavLink>
        </nav>
      </aside>
      <main className="main-content">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/files" element={<Files />} />
          <Route path="/tasks" element={<Tasks />} />
          <Route path="/flows" element={<Flows />} />
          <Route path="/flows/:id" element={<FlowDetail />} />
          <Route path="/jobs" element={<Jobs />} />
          <Route path="/discover" element={<Discover />} />
          <Route path="/symbols/:id" element={<SymbolDetail />} />
        </Routes>
      </main>
    </div>
  );
}
