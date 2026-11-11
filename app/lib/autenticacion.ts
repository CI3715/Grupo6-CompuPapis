import { invoke } from "@tauri-apps/api/core";

export type UsuarioAuth = {
  id: string;
  nombre_usuario: string;
  nombre_visible: string;
  correo: string;
  fecha_alta: string;
};
export type ErrorAuth = { campo: string; mensaje: string };
export type RegistroAuth = {
  nombre_usuario: string;
  nombre_visible: string;
  correo: string;
  contrasena: string;
};
export type CredencialesAuth = { correo: string; contrasena: string };
export type SesionAuth = { usuario: UsuarioAuth; vencimiento: string };

export const registrarUsuario = (datos: RegistroAuth) =>
  invoke<UsuarioAuth>("registrar_usuario", { datos });
export const iniciarSesion = (datos: CredencialesAuth) =>
  invoke<SesionAuth>("iniciar_sesion", { datos });
export const obtenerPerfil = () => invoke<UsuarioAuth>("obtener_perfil");
export const cerrarSesion = () => invoke<void>("cerrar_sesion");
