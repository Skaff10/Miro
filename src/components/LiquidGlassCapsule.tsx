import {
  forwardRef,
  useCallback,
  useRef,
  type ButtonHTMLAttributes,
  type HTMLAttributes,
  type KeyboardEvent,
  type PointerEvent,
  type ReactNode,
} from "react";

export type GlassSurfaceVariant = "panel" | "card" | "compact" | "focus" | "toast";
export type GlassCapsuleVariant = GlassSurfaceVariant;

interface BaseProps {
  variant?: GlassSurfaceVariant;
  interactive?: boolean;
  /** Marks this surface as a top-level window region backed by Windows DWM HostBackdropBrush. */
  nativeBackdrop?: boolean;
  /** Blur sigma passed to the native Windows.UI.Composition GaussianBlur effect. */
  backdropBlur?: number;
  className?: string;
  children?: ReactNode;
}

type DivSurfaceProps = BaseProps &
  Omit<HTMLAttributes<HTMLDivElement>, keyof BaseProps> & {
    as?: "div";
  };

type ButtonSurfaceProps = BaseProps &
  Omit<ButtonHTMLAttributes<HTMLButtonElement>, keyof BaseProps> & {
    as: "button";
  };

export type LiquidGlassCapsuleProps = DivSurfaceProps | ButtonSurfaceProps;

function prefersReducedMotion(): boolean {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return false;
  }
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function triggerGlassBloom(
  bloomEl: HTMLElement | null,
  hostEl: HTMLElement | null,
  clientX: number,
  clientY: number
): void {
  if (!bloomEl || !hostEl || prefersReducedMotion()) {
    return;
  }
  const bounds = hostEl.getBoundingClientRect();
  if (bounds.width <= 0 || bounds.height <= 0) {
    return;
  }
  const x = clientX - bounds.left;
  const y = clientY - bounds.top;
  const size = Math.max(bounds.width, bounds.height) * 2.15;

  bloomEl.style.setProperty("--lg-bloom-x", `${x}px`);
  bloomEl.style.setProperty("--lg-bloom-y", `${y}px`);
  bloomEl.style.setProperty("--lg-bloom-size", `${size}px`);
  bloomEl.animate?.(
    [
      { opacity: 0, transform: "scale(0.1)" },
      { opacity: 1, transform: "scale(0.45)", offset: 0.18 },
      { opacity: 0, transform: "scale(1)" },
    ],
    { duration: 560, easing: "cubic-bezier(0.22, 0.8, 0.3, 1)" }
  );
}

function defaultNativeBackdrop(variant: GlassSurfaceVariant): boolean {
  return variant !== "card";
}

function defaultBackdropBlur(variant: GlassSurfaceVariant): number {
  switch (variant) {
    case "panel":
      return 16;
    case "toast":
      return 13.5;
    case "compact":
      return 12;
    case "focus":
      return 10;
    default:
      return 10;
  }
}

/**
 * Unified Liquid Glass surface shared across Normal, Compact, Focus, Card, and Toast layers.
 * Pairs native DWM HostBackdropBrush blur/saturation (on top-level desktop regions)
 * with curved meniscus reflections, prismatic rim dispersion, masked directional specular ring,
 * micro-grain, and tactile pointer light bloom.
 */
export const LiquidGlassCapsule = forwardRef<HTMLElement, LiquidGlassCapsuleProps>(
  function LiquidGlassCapsule(props, forwardedRef) {
    const localRef = useRef<HTMLElement | null>(null);
    const bloomRef = useRef<HTMLSpanElement | null>(null);

    const setRef = useCallback(
      (node: HTMLElement | null) => {
        localRef.current = node;
        if (typeof forwardedRef === "function") {
          forwardedRef(node);
        } else if (forwardedRef) {
          forwardedRef.current = node;
        }
      },
      [forwardedRef]
    );

    const updateGlintFromPointer = useCallback((el: HTMLElement, clientX: number) => {
      if (prefersReducedMotion()) return;
      const rect = el.getBoundingClientRect();
      if (rect.width <= 0) return;
      const ratio = Math.min(1, Math.max(0, (clientX - rect.left) / rect.width));
      const glintX = 16 + ratio * 38;
      el.style.setProperty("--miro-glint-x", `${glintX.toFixed(1)}%`);
    }, []);

    const resetGlint = useCallback((el: HTMLElement) => {
      el.style.removeProperty("--miro-glint-x");
    }, []);

    if (props.as === "button") {
      const {
        as: _as,
        variant = "focus",
        interactive = true,
        nativeBackdrop = defaultNativeBackdrop(variant),
        backdropBlur = defaultBackdropBlur(variant),
        className = "",
        children,
        type = "button",
        onPointerDown,
        onPointerUp,
        onPointerMove,
        onPointerLeave,
        onPointerCancel,
        onBlur,
        onKeyDown,
        onKeyUp,
        ...rest
      } = props;

      const handlePointerDown = (e: PointerEvent<HTMLButtonElement>) => {
        onPointerDown?.(e);
        if (e.defaultPrevented || !interactive) return;
        const el = e.currentTarget;
        el.dataset.pressed = "true";
        triggerGlassBloom(bloomRef.current, el, e.clientX, e.clientY);
      };

      const release = (el: HTMLButtonElement) => {
        el.dataset.pressed = "false";
      };

      const handleKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
        onKeyDown?.(e);
        if (e.defaultPrevented || !interactive) return;
        if (e.key === " " || e.key === "Enter") {
          const el = e.currentTarget;
          el.dataset.pressed = "true";
          if (!e.repeat) {
            const rect = el.getBoundingClientRect();
            triggerGlassBloom(
              bloomRef.current,
              el,
              rect.left + rect.width / 2,
              rect.top + rect.height / 2
            );
          }
        }
      };

      return (
        <button
          ref={setRef as React.Ref<HTMLButtonElement>}
          type={type}
          data-variant={variant}
          data-interactive={interactive ? "true" : "false"}
          data-pressed="false"
          data-glass-backdrop={nativeBackdrop ? "true" : undefined}
          data-glass-blur={nativeBackdrop ? String(backdropBlur) : undefined}
          className={`miro-glass-capsule ${className}`}
          onPointerDown={handlePointerDown}
          onPointerMove={(e) => {
            onPointerMove?.(e);
            if (interactive && variant !== "focus") {
              updateGlintFromPointer(e.currentTarget, e.clientX);
            }
          }}
          onPointerUp={(e) => {
            onPointerUp?.(e);
            release(e.currentTarget);
          }}
          onPointerLeave={(e) => {
            onPointerLeave?.(e);
            release(e.currentTarget);
            resetGlint(e.currentTarget);
          }}
          onPointerCancel={(e) => {
            onPointerCancel?.(e);
            release(e.currentTarget);
          }}
          onBlur={(e) => {
            onBlur?.(e);
            release(e.currentTarget);
          }}
          onKeyDown={handleKeyDown}
          onKeyUp={(e) => {
            onKeyUp?.(e);
            if (e.key === " " || e.key === "Enter") {
              release(e.currentTarget);
            }
          }}
          {...rest}
        >
          <span className="miro-glass-noise" aria-hidden="true" />
          <span ref={bloomRef} className="miro-glass-bloom" aria-hidden="true" />
          {children}
        </button>
      );
    }

    const {
      as: _as,
      variant = "compact",
      interactive = true,
      nativeBackdrop = defaultNativeBackdrop(variant),
      backdropBlur = defaultBackdropBlur(variant),
      className = "",
      children,
      onPointerMove,
      onPointerLeave,
      ...rest
    } = props;

    return (
      <div
        ref={setRef as React.Ref<HTMLDivElement>}
        data-variant={variant}
        data-interactive={interactive ? "true" : "false"}
        data-pressed="false"
        data-glass-backdrop={nativeBackdrop ? "true" : undefined}
        data-glass-blur={nativeBackdrop ? String(backdropBlur) : undefined}
        className={`miro-glass-capsule ${className}`}
        onPointerMove={(e) => {
          onPointerMove?.(e);
          if (interactive) {
            updateGlintFromPointer(e.currentTarget, e.clientX);
          }
        }}
        onPointerLeave={(e) => {
          onPointerLeave?.(e);
          resetGlint(e.currentTarget);
        }}
        {...rest}
      >
        <span className="miro-glass-noise" aria-hidden="true" />
        <span
          ref={bloomRef}
          data-glass-bloom="true"
          className="miro-glass-bloom"
          aria-hidden="true"
        />
        {children}
      </div>
    );
  }
);

export const LiquidGlassSurface = LiquidGlassCapsule;
