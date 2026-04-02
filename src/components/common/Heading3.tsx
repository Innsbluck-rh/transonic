import { Component, JSX } from 'solid-js';

const Heading3: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p {...props} class={`archivo-black w-fit origin-top-left scale-x-150 text-[9px] tracking-tighter italic ${props.class}`}>
      {props.children}
    </p>
  );
};

export default Heading3;
