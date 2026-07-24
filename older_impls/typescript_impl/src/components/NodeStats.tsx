import React from "react";
import "./styles/NodeStats.css";

interface Props {
  value: number;
  rollouts: number;
}

const DIGITS_AFTER_DECIMAL = 3;

export default function NodeStats({
  value,
  rollouts,
}: Props): React.ReactElement {
  const meanValue = value / rollouts;
  return (
    <div className="NodeStats">
      [v̅ = {meanValue.toFixed(DIGITS_AFTER_DECIMAL)}, n = {rollouts}]
    </div>
  );
}
