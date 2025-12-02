import {
  type RouteConfig,
  index,
  route,
} from "@react-router/dev/routes";

export default [
  // The home page, for uploading a document.
  // It will be rendered inside the `root.tsx` layout.
  index("./routes/home.tsx"),
  
  route("login", "./routes/login.tsx"),
  // The main session/player page.
  route("sessions/:id", "./routes/session.tsx"),


] satisfies RouteConfig;

