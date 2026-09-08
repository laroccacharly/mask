import { expect, test } from "@playwright/test"

test("encodes sensitive text", async ({ page }) => {
  await page.goto("/")

  const input = page.getByLabel("Input")
  const output = page.getByLabel("Output")
  const text =
    "Marie Tremblay works at Acme Robotics Corp in Montreal. Email marie@example.com."

  await input.fill(text)
  await page.getByRole("button", { name: "Encode" }).click()

  await expect(output).not.toHaveText("")
  await expect(output).not.toHaveText(text)
  await expect(output).not.toHaveText(/Marie Tremblay/)
  await expect(output).not.toHaveText(/marie@example\.com/)
  await expect(output).toHaveText(/<[A-Za-z]+_\d+>/)
  await expect(page.getByRole("status")).toHaveText(/^Encoded in \d+ ms$/)
})

test("decodes encoded text", async ({ page }) => {
  await page.goto("/")

  const text = "Marie Tremblay can be reached at marie@example.com."
  await page.getByLabel("Input").fill(text)
  await page.getByRole("button", { name: "Encode" }).click()
  const encodeOutput = page.getByLabel("Output")
  await expect(encodeOutput).not.toHaveText("")
  const encoded = (await encodeOutput.textContent()) ?? ""

  await page.getByRole("link", { name: /^Decode/ }).click()
  await expect(page).toHaveURL(/#\/decode$/)
  await expect(page.getByRole("heading", { name: "Decode" })).toBeVisible()
  await page.getByLabel("Input").fill(encoded)
  await page.getByRole("button", { name: "Decode" }).click()

  await expect(page.getByLabel("Output")).toHaveText(
    "marie tremblay can be reached at marie@example.com."
  )
  await expect(page.getByRole("status")).toHaveText(/^Decoded in \d+ ms$/)
})

test("shows an error when the input is empty", async ({ page }) => {
  await page.goto("/")

  await page.getByRole("button", { name: "Encode" }).click()

  await expect(page.getByRole("alert")).toHaveText("Input cannot be empty")
  await expect(page.getByLabel("Output")).toHaveCount(0)
})
