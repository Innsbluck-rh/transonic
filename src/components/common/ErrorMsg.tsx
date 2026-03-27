import { Component, JSX } from 'solid-js';

const ErrorMsg: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p class='text-md text-red-500' {...props}>
      {props.children}
    </p>
  );
};

export default ErrorMsg;
