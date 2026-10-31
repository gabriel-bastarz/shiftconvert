import { useLayoutEffect, useRef, useState } from "react";

export interface ElementSize {
  width: number;
  height: number;
}

/**
 * Mede um elemento para que a roda radial seja desenhada em unidades CSS
 * reais. Assim os rótulos mantêm o mesmo tamanho aparente em qualquer
 * resolução, em vez de serem reduzidos por um viewBox fixo.
 */
export function useElementSize<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  const [size, setSize] = useState<ElementSize>({ width: 0, height: 0 });

  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;

    const update = () => {
      const rect = element.getBoundingClientRect();
      setSize((current) => {
        const width = Math.round(rect.width);
        const height = Math.round(rect.height);
        if (current.width === width && current.height === height) {
          return current;
        }
        return { width, height };
      });
    };

    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  return [ref, size] as const;
}
