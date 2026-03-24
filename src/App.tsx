// @refresh reload

import { MetaProvider } from "@solidjs/meta";
import { Route, Router } from "@solidjs/router";
import Home from "./routes/home";
import InitLogin from "./routes/init_login";
import Index from "./routes";

export default function App() {
  return (
    <Router
      root={(props) => (
        <MetaProvider>
          <title>transonic</title>
          <div>
            <main>{props.children}</main>
          </div>
        </MetaProvider>
      )}
    >
      <Route path="/" component={Index} />
      <Route path="/home" component={Home} />
      <Route path="/init_login" component={InitLogin} />
    </Router>
  );
}
