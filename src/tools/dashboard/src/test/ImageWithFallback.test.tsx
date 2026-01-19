import React from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import { ImageWithFallback } from "../components/common/ImageWithFallback";

describe("ImageWithFallback", () => {
  it("renders the image with given src and alt", () => {
    render(<ImageWithFallback src="test.jpg" alt="test image" data-testid="img" />);
    const img = screen.getByTestId("img");
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute("src", "test.jpg");
    expect(img).toHaveAttribute("alt", "test image");
  });

  it("shows fallback image on error", () => {
    render(<ImageWithFallback src="broken.jpg" alt="broken image" data-testid="img" />);
    const img = screen.getByTestId("img");
    // Simulate error
    fireEvent.error(img);
    // Now fallback image should be rendered
    const fallbackImg = screen.getByAltText("Error loading image");
    expect(fallbackImg).toBeInTheDocument();
    expect(fallbackImg).toHaveAttribute("data-original-url", "broken.jpg");
  });
});
