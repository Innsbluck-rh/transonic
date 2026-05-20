import { Component, JSX } from 'solid-js';

const Heading3: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p {...props} class={`h-fit w-fit origin-top-left scale-x-150 text-xs leading-3 font-black tracking-tighter ${props.class}`}>
      {props.children}
    </p>
  );
};

export default Heading3;
