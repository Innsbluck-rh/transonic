import { Component, JSX } from 'solid-js';

const Heading2: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p {...props} class={`archivo-black text-xs w-fit origin-top-left scale-x-150 italic tracking-tighter ${props.class}`}>
      {props.children}
    </p>
  );
};

export default Heading2;
