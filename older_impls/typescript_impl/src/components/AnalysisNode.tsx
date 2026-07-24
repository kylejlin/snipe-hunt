import React from "react";
import { isAnimalStep } from "../gameUtil";
import {
  MctsAnalyzer,
  NodePointer,
  NodeSummary,
  pointerToIndex,
} from "../mcts";
import { SuggestionDetailLevel } from "../types";
import InlineAtomic, { Ellipsis } from "./InlineAtomic";
import NodeStats from "./NodeStats";

interface Props {
  analyzer: MctsAnalyzer;
  suggestionDetailLevels: {
    [pointer: number]: SuggestionDetailLevel | undefined;
  };
  plyNumber: number;
  isTherePendingAnimalStep: boolean;
  viewedNode: NodeSummary;
  onDetailLevelChange(pointer: NodePointer, level: SuggestionDetailLevel): void;
}

interface SuggestionDetailLevelMenuProps {
  pointer: NodePointer;
  detailLevel: SuggestionDetailLevel;
  onChange(pointer: NodePointer, level: SuggestionDetailLevel): void;
}

export default function AnalysisNode({
  analyzer,
  suggestionDetailLevels,
  plyNumber,
  isTherePendingAnimalStep,
  viewedNode,
  onDetailLevelChange,
}: Props): React.ReactElement {
  const optDetailLevel =
    suggestionDetailLevels[pointerToIndex(viewedNode.pointer)];
  const detailLevel: SuggestionDetailLevel =
    optDetailLevel === undefined ? SuggestionDetailLevel.None : optDetailLevel;

  const pointersToConsideredChildren: NodePointer[] = (() => {
    const pointers = analyzer.getChildPointersFromBestToWorst(
      viewedNode.pointer
    );

    switch (detailLevel) {
      case SuggestionDetailLevel.None:
        return [];

      case SuggestionDetailLevel.BestAction:
        return pointers.slice(0, 1);

      case SuggestionDetailLevel.AllActions:
        return pointers;
    }
  })();

  return (
    <div>
      {viewedNode.atomic.match({
        none: () => "Current state:",
        some: (atomic) => (
          <InlineAtomic
            atomic={atomic}
            plyNumber={plyNumber}
            ellipsis={
              isTherePendingAnimalStep ? Ellipsis.Before : Ellipsis.After
            }
          />
        ),
      })}{" "}
      <NodeStats value={viewedNode.value} rollouts={viewedNode.rollouts} />{" "}
      <SuggestionDetailLevelMenu
        pointer={viewedNode.pointer}
        detailLevel={detailLevel}
        onChange={onDetailLevelChange}
      />
      <ol>
        {pointersToConsideredChildren.map((childPointer, i) => {
          const childSummary = analyzer.getNodeSummary(childPointer);
          const {
            plyNumberAfterPerformingChildAtomic,
            isTherePendingAnimalStepAfterPerformingChildAtomic,
          } = ((): {
            plyNumberAfterPerformingChildAtomic: number;
            isTherePendingAnimalStepAfterPerformingChildAtomic: boolean;
          } => {
            if (viewedNode.atomic.someSatisfies(isAnimalStep)) {
              if (isTherePendingAnimalStep) {
                return {
                  plyNumberAfterPerformingChildAtomic: plyNumber + 1,
                  isTherePendingAnimalStepAfterPerformingChildAtomic: false,
                };
              } else {
                return {
                  plyNumberAfterPerformingChildAtomic: plyNumber,
                  isTherePendingAnimalStepAfterPerformingChildAtomic: true,
                };
              }
            } else {
              return {
                plyNumberAfterPerformingChildAtomic: plyNumber + 1,
                isTherePendingAnimalStepAfterPerformingChildAtomic: false,
              };
            }
          })();

          return (
            <li key={i}>
              <AnalysisNode
                analyzer={analyzer}
                suggestionDetailLevels={suggestionDetailLevels}
                plyNumber={plyNumberAfterPerformingChildAtomic}
                isTherePendingAnimalStep={
                  isTherePendingAnimalStepAfterPerformingChildAtomic
                }
                viewedNode={childSummary}
                onDetailLevelChange={onDetailLevelChange}
              />
            </li>
          );
        })}
      </ol>
    </div>
  );
}

function SuggestionDetailLevelMenu({
  pointer,
  detailLevel,
  onChange,
}: SuggestionDetailLevelMenuProps): React.ReactElement {
  function selectOnChange(event: React.ChangeEvent<HTMLSelectElement>): void {
    const level = event.target.value as SuggestionDetailLevel;
    onChange(pointer, level);
  }

  return (
    <>
      Display{" "}
      <select value={detailLevel} onChange={selectOnChange}>
        <option value={SuggestionDetailLevel.None}>no actions</option>
        <option value={SuggestionDetailLevel.BestAction}>best action</option>
        <option value={SuggestionDetailLevel.AllActions}>all actions</option>
      </select>
    </>
  );
}
