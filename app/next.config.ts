import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "export",

  experimental: {
    agentFeedback: true,
    // TypeScript 5: comprobar tipos mediante su API sin depender de stdout de un subproceso.
    useTypeScriptCli: false,
  },
};

export default nextConfig;
