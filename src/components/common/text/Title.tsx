import { Component, JSX } from 'solid-js';

const Title: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p {...props} class={`archivo-black w-fit origin-top-left scale-x-150 text-xl font-black tracking-tighter italic ${props.class}`}>
      {props.children}
    </p>
  );
};

export default Title;
